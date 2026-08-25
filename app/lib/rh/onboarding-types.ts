import type { RhCategory, RhContractType } from "@/app/lib/rh/types";

export type RhOnboardingRecordStatus =
  | "awaiting_candidate"
  | "submitted"
  | "validation_rh"
  | "provisioned"
  | "active"
  | "cancelled";

/** Données saisies par le nouvel arrivant (formulaire public). */
export type RhOnboardingFormData = {
  firstName: string;
  lastName: string;
  birthName?: string | null;
  email: string;
  phone?: string | null;
  phoneMobile?: string | null;
  birthDate: string;
  birthPlace: string;
  birthDepartment?: string | null;
  nationality?: string | null;
  gender?: "M" | "F" | "autre" | null;
  socialSecurityNumber: string;
  addressLine1: string;
  addressLine2?: string | null;
  postalCode: string;
  city: string;
  country?: string | null;
  maritalStatus?: string | null;
  childrenCount?: number | null;
  iban?: string | null;
  bic?: string | null;
  emergencyContactName?: string | null;
  emergencyContactPhone?: string | null;
  contractType: RhContractType;
  jobTitle: string;
  etablissement?: string | null;
  contractStartDate: string;
  contractEndDate?: string | null;
  workTimePercent?: number | null;
  classification?: string | null;
  coefficient?: string | null;
  grossMonthlySalary?: string | null;
  category?: RhCategory | null;
  notes?: string | null;
};

export type RhOnboardingRecord = {
  id: string;
  token: string;
  status: RhOnboardingRecordStatus;
  createdAt: string;
  updatedAt: string;
  expiresAt: string;
  submittedAt?: string | null;
  validatedAt?: string | null;
  activatedAt?: string | null;
  createdBy: { userId: string; name: string; email?: string | null };
  candidateEmailHint?: string | null;
  form?: RhOnboardingFormData | null;
  personnelId?: string | null;
  folderName?: string | null;
  externalUserId?: string | null;
  invitePending?: boolean;
  publicPath?: string | null;
  validationNote?: string | null;
  validatedBy?: string | null;
};

export type RhOnboardingIndexEntry = {
  id: string;
  token: string;
  status: RhOnboardingRecordStatus;
  createdAt: string;
  submittedAt?: string | null;
  displayName?: string | null;
  email?: string | null;
};

export type RhOnboardingIndex = {
  version: 1;
  updatedAt: string;
  entries: RhOnboardingIndexEntry[];
};

export const RH_ONBOARDING_INDEX_KEY = "rh/onboarding/index.json";
export const RH_ONBOARDING_TTL_MS = 30 * 24 * 60 * 60 * 1000;

export function rhOnboardingRecordKey(id: string) {
  return `rh/onboarding/records/${id}.json`;
}

export function rhOnboardingTokenKey(token: string) {
  return `rh/onboarding/tokens/${token}.json`;
}

export const RH_ONBOARDING_STATUS_LABELS: Record<RhOnboardingRecordStatus, string> = {
  awaiting_candidate: "En attente du candidat",
  submitted: "Formulaire reçu",
  validation_rh: "Validation RH",
  provisioned: "Dossier créé · invitation en attente",
  active: "Actif",
  cancelled: "Annulé",
};

export function normalizeRhOnboardingForm(raw: unknown): RhOnboardingFormData | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  const str = (v: unknown) => (typeof v === "string" ? v.trim() : "");
  const firstName = str(o.firstName);
  const lastName = str(o.lastName);
  const email = str(o.email).toLowerCase();
  const birthDate = str(o.birthDate);
  const birthPlace = str(o.birthPlace);
  const socialSecurityNumber = str(o.socialSecurityNumber);
  const addressLine1 = str(o.addressLine1);
  const postalCode = str(o.postalCode);
  const city = str(o.city);
  const contractType = str(o.contractType) as RhContractType;
  const jobTitle = str(o.jobTitle);
  const contractStartDate = str(o.contractStartDate);

  if (!firstName || !lastName || !email || !birthDate || !birthPlace || !socialSecurityNumber) {
    return null;
  }
  if (!addressLine1 || !postalCode || !city || !jobTitle || !contractStartDate) return null;
  if (!["cdi", "cdd", "cddu", "interim", "stage", "autre"].includes(contractType)) return null;

  return {
    firstName,
    lastName,
    birthName: str(o.birthName) || null,
    email,
    phone: str(o.phone) || null,
    phoneMobile: str(o.phoneMobile) || null,
    birthDate,
    birthPlace,
    birthDepartment: str(o.birthDepartment) || null,
    nationality: str(o.nationality) || "Française",
    gender:
      o.gender === "M" || o.gender === "F" || o.gender === "autre" ? o.gender : null,
    socialSecurityNumber,
    addressLine1,
    addressLine2: str(o.addressLine2) || null,
    postalCode,
    city,
    country: str(o.country) || "France",
    maritalStatus: str(o.maritalStatus) || null,
    childrenCount:
      typeof o.childrenCount === "number" && Number.isFinite(o.childrenCount)
        ? o.childrenCount
        : null,
    iban: str(o.iban) || null,
    bic: str(o.bic) || null,
    emergencyContactName: str(o.emergencyContactName) || null,
    emergencyContactPhone: str(o.emergencyContactPhone) || null,
    contractType,
    jobTitle,
    etablissement: str(o.etablissement) || null,
    contractStartDate,
    contractEndDate: str(o.contractEndDate) || null,
    workTimePercent:
      typeof o.workTimePercent === "number" && Number.isFinite(o.workTimePercent)
        ? o.workTimePercent
        : 100,
    classification: str(o.classification) || null,
    coefficient: str(o.coefficient) || null,
    grossMonthlySalary: str(o.grossMonthlySalary) || null,
    category:
      o.category === "administratif" ||
      o.category === "surveillant" ||
      o.category === "cpe" ||
      o.category === "comptabilite" ||
      o.category === "maintenance" ||
      o.category === "professeur" ||
      o.category === "direction"
        ? o.category
        : "administratif",
    notes: str(o.notes) || null,
  };
}

export function normalizeRhOnboardingRecord(raw: unknown): RhOnboardingRecord | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  const id = String(o.id ?? "").trim();
  const token = String(o.token ?? "").trim();
  if (!id || !token) return null;
  const status = String(o.status ?? "") as RhOnboardingRecordStatus;
  const validStatuses: RhOnboardingRecordStatus[] = [
    "awaiting_candidate",
    "submitted",
    "validation_rh",
    "provisioned",
    "active",
    "cancelled",
  ];
  if (!validStatuses.includes(status)) return null;

  const createdByRaw =
    o.createdBy && typeof o.createdBy === "object"
      ? (o.createdBy as Record<string, unknown>)
      : {};

  return {
    id,
    token,
    status,
    createdAt: String(o.createdAt ?? new Date().toISOString()),
    updatedAt: String(o.updatedAt ?? new Date().toISOString()),
    expiresAt: String(o.expiresAt ?? new Date().toISOString()),
    submittedAt: o.submittedAt ? String(o.submittedAt) : null,
    validatedAt: o.validatedAt ? String(o.validatedAt) : null,
    activatedAt: o.activatedAt ? String(o.activatedAt) : null,
    createdBy: {
      userId: String(createdByRaw.userId ?? ""),
      name: String(createdByRaw.name ?? "RH"),
      email: createdByRaw.email ? String(createdByRaw.email) : null,
    },
    candidateEmailHint: o.candidateEmailHint ? String(o.candidateEmailHint) : null,
    form: o.form ? normalizeRhOnboardingForm(o.form) : null,
    personnelId: o.personnelId ? String(o.personnelId) : null,
    folderName: o.folderName ? String(o.folderName) : null,
    externalUserId: o.externalUserId ? String(o.externalUserId) : null,
    invitePending: o.invitePending === true,
    publicPath: o.publicPath ? String(o.publicPath) : null,
    validationNote: o.validationNote ? String(o.validationNote) : null,
    validatedBy: o.validatedBy ? String(o.validatedBy) : null,
  };
}

export function isOnboardingTokenUsable(record: RhOnboardingRecord): boolean {
  if (record.status === "cancelled") return false;
  if (new Date(record.expiresAt).getTime() < Date.now()) return false;
  return record.status === "awaiting_candidate" || record.status === "submitted";
}

export function onboardingEntryFromRecord(r: RhOnboardingRecord): RhOnboardingIndexEntry {
  const displayName = r.form
    ? `${r.form.firstName} ${r.form.lastName}`.trim()
    : r.candidateEmailHint || null;
  return {
    id: r.id,
    token: r.token,
    status: r.status,
    createdAt: r.createdAt,
    submittedAt: r.submittedAt ?? null,
    displayName,
    email: r.form?.email ?? r.candidateEmailHint ?? null,
  };
}
