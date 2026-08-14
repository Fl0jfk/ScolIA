/**
 * Types RH Core — source de vérité OneDrive (`meta-rh.json` + `personnel-index.json`).
 * Les modules absences / HSE restent sur leurs types S3 existants (liaison via personnelId).
 */

import {
  buildEleveFolderName,
  formatEleveNomForFolder,
  formatElevePrenomForFolder,
} from "@/app/lib/eleves-config";
import { RH_DEFAULT_BASE_PATH } from "@/app/lib/rh/paths";

export type RhCategory =
  | "administratif"
  | "maintenance"
  | "education"
  | "cpe"
  | "comptabilite"
  | "professeur"
  | "direction";

export type RhAccountStatus = "pending" | "active" | "suspended";

export type RhContractType = "cdi" | "cdd" | "cddu" | "interim" | "stage" | "autre";

export type RhDocCategory =
  | "contrat"
  | "formation"
  | "habilitation"
  | "medecine"
  | "entretien"
  | "onboarding"
  | "personnel"
  | "autre";

export type RhDocumentRef = {
  id: string;
  name: string;
  /** Chemin relatif OneDrive depuis la racine du dossier collab (ex. documents/contrats/…). */
  oneDrivePath: string;
  category: RhDocCategory;
  uploadedAt: string;
  uploadedBy: string;
  expiresAt?: string | null;
};

type RhFormationStatus = "demandee" | "planifiee" | "realisee" | "annulee";

export type RhFormation = {
  id: string;
  title: string;
  status: RhFormationStatus;
  plannedDate?: string | null;
  completedDate?: string | null;
  reminderAt?: string | null;
  documentId?: string | null;
  notes?: string;
};

export type RhHabilitation = {
  id: string;
  label: string;
  obtainedAt?: string | null;
  expiresAt: string;
  documentId?: string | null;
  notes?: string;
};

type RhMedecineVisit = {
  id: string;
  visitedAt: string;
  visitType?: string;
  documentId?: string | null;
  notes?: string;
  createdAt: string;
};

export type RhMedecineTravail = {
  visits?: RhMedecineVisit[];
  lastVisitAt?: string | null;
  nextVisitAt?: string | null;
  visitType?: string;
  documentId?: string | null;
  notes?: string;
};

type RhEntretienStatus = "a_planifier" | "planifie" | "realise";

export type RhEntretien = {
  id: string;
  scheduledAt?: string | null;
  completedAt?: string | null;
  reminderAt?: string | null;
  nextDueAt?: string | null;
  documentId?: string | null;
  notes?: string;
  status: RhEntretienStatus;
};

type RhOnboardingStatus =
  | "brouillon"
  | "soumis"
  | "validation_rh"
  | "documents"
  | "signatures"
  | "termine"
  | "annule";

export type RhOnboardingState = {
  id: string;
  status: RhOnboardingStatus;
  publicToken?: string | null;
  submittedAt?: string | null;
  startDate?: string | null;
  notes?: string;
};

export type RhIdentity = {
  firstName: string;
  lastName: string;
  birthName?: string | null;
  email: string;
  phone?: string | null;
  phoneMobile?: string | null;
  birthDate?: string | null;
  birthPlace?: string | null;
  nationality?: string | null;
  gender?: "M" | "F" | "autre" | null;
  socialSecurityNumber?: string | null;
  address?: {
    line1?: string | null;
    line2?: string | null;
    postalCode?: string | null;
    city?: string | null;
    country?: string | null;
  };
};

export type RhContract = {
  type?: RhContractType | null;
  startDate?: string | null;
  endDate?: string | null;
  jobTitle?: string | null;
  etablissement?: string | null;
  workTimePercent?: number | null;
  classification?: string | null;
  coefficient?: string | null;
  grossMonthlySalary?: string | null;
};

export type RhComplianceFlags = {
  missingSocialSecurity?: boolean;
  missingContractType?: boolean;
  missingBirthDate?: boolean;
};

/** Index OneDrive : Dossier personnel/personnel-index.json */
export type RhPersonnelIndex = {
  version: 1;
  updatedAt: string;
  basePath: string;
  entries: RhPersonnelIndexEntry[];
};

export type RhPersonnelIndexEntry = {
  id: string;
  folderName: string;
  displayName: string;
  email: string;
  clerkUserId?: string | null;
  category: RhCategory;
  active: boolean;
  accountStatus: RhAccountStatus;
  hireDate?: string | null;
};

/** Racine de chaque dossier : meta-rh.json */
export type MetaRhDocument = {
  version: 1;
  id: string;
  identity: RhIdentity;
  contract: RhContract;
  banking?: { iban?: string | null; bic?: string | null };
  emergencyContact?: { name?: string | null; phone?: string | null };
  clerkUserId?: string | null;
  accountStatus: RhAccountStatus;
  category: RhCategory;
  active: boolean;
  hireDate?: string | null;
  documents: RhDocumentRef[];
  formations: RhFormation[];
  habilitations: RhHabilitation[];
  medecineTravail: RhMedecineTravail;
  entretiens: RhEntretien[];
  onboarding?: RhOnboardingState | null;
  complianceFlags?: RhComplianceFlags;
  updatedAt: string;
  updatedBy?: string;
};

export const RH_CATEGORY_LABELS: Record<RhCategory, string> = {
  administratif: "Administratif",
  maintenance: "Maintenance",
  education: "Éducation / surveillance",
  cpe: "CPE",
  comptabilite: "Comptabilité",
  professeur: "Professeur",
  direction: "Direction",
};

const RH_CATEGORIES = Object.keys(RH_CATEGORY_LABELS) as RhCategory[];

export function buildPersonnelFolderName(lastName: string, firstName: string): string {
  return buildEleveFolderName(lastName, firstName);
}

function formatPersonnelLastName(lastName: string): string {
  return formatEleveNomForFolder(lastName);
}

function formatPersonnelFirstName(firstName: string): string {
  return formatElevePrenomForFolder(firstName);
}

export function rhUid(prefix = "rh") {
  return `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`;
}

export function computeRhComplianceFlags(meta: Pick<MetaRhDocument, "identity" | "contract">): RhComplianceFlags {
  return {
    missingSocialSecurity: !meta.identity.socialSecurityNumber?.trim(),
    missingContractType: !meta.contract.type,
    missingBirthDate: !meta.identity.birthDate?.trim(),
  };
}

function emptyMedecineTravail(): RhMedecineTravail {
  return { visits: [], lastVisitAt: null, nextVisitAt: null, visitType: "", notes: "" };
}

export function createEmptyMetaRh(input: {
  id?: string;
  firstName: string;
  lastName: string;
  email: string;
  category?: RhCategory;
  accountStatus?: RhAccountStatus;
}): MetaRhDocument {
  const firstName = input.firstName.trim();
  const lastName = input.lastName.trim();
  const identity: RhIdentity = {
    firstName,
    lastName,
    email: input.email.trim().toLowerCase(),
    nationality: "Française",
    address: { country: "France" },
  };
  const contract: RhContract = { workTimePercent: 100 };
  const now = new Date().toISOString();
  const draft: MetaRhDocument = {
    version: 1,
    id: input.id || rhUid(),
    identity,
    contract,
    banking: {},
    emergencyContact: {},
    clerkUserId: null,
    accountStatus: input.accountStatus ?? "pending",
    category: input.category ?? "administratif",
    active: true,
    hireDate: null,
    documents: [],
    formations: [],
    habilitations: [],
    medecineTravail: emptyMedecineTravail(),
    entretiens: [],
    onboarding: null,
    complianceFlags: computeRhComplianceFlags({ identity, contract }),
    updatedAt: now,
  };
  return draft;
}

export function emptyRhPersonnelIndex(basePath = RH_DEFAULT_BASE_PATH): RhPersonnelIndex {
  return {
    version: 1,
    updatedAt: new Date().toISOString(),
    basePath: basePath.trim() || RH_DEFAULT_BASE_PATH,
    entries: [],
  };
}

export function metaToIndexEntry(meta: MetaRhDocument): RhPersonnelIndexEntry {
  return {
    id: meta.id,
    folderName: buildPersonnelFolderName(meta.identity.lastName, meta.identity.firstName),
    displayName: `${meta.identity.firstName} ${meta.identity.lastName}`.trim(),
    email: meta.identity.email,
    clerkUserId: meta.clerkUserId ?? null,
    category: meta.category,
    active: meta.active !== false,
    accountStatus: meta.accountStatus,
    hireDate: meta.hireDate ?? null,
  };
}

function asCategory(v: unknown): RhCategory {
  return RH_CATEGORIES.includes(v as RhCategory) ? (v as RhCategory) : "administratif";
}

function asAccountStatus(v: unknown): RhAccountStatus {
  return v === "pending" || v === "active" || v === "suspended" ? v : "pending";
}

export function normalizeMetaRhDocument(raw: unknown): MetaRhDocument {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const identityRaw =
    o.identity && typeof o.identity === "object" ? (o.identity as Record<string, unknown>) : {};
  const contractRaw =
    o.contract && typeof o.contract === "object" ? (o.contract as Record<string, unknown>) : {};
  const str = (v: unknown) => (typeof v === "string" ? v : "");
  const firstName = str(identityRaw.firstName).trim() || str(o.firstName).trim();
  const lastName = str(identityRaw.lastName).trim() || str(o.lastName).trim();
  const email = (str(identityRaw.email) || str(o.email)).trim().toLowerCase();

  const identity: RhIdentity = {
    firstName,
    lastName,
    birthName: str(identityRaw.birthName) || null,
    email,
    phone: str(identityRaw.phone) || null,
    phoneMobile: str(identityRaw.phoneMobile) || null,
    birthDate: str(identityRaw.birthDate) || null,
    birthPlace: str(identityRaw.birthPlace) || null,
    nationality: str(identityRaw.nationality) || null,
    gender:
      identityRaw.gender === "M" || identityRaw.gender === "F" || identityRaw.gender === "autre"
        ? identityRaw.gender
        : null,
    socialSecurityNumber: str(identityRaw.socialSecurityNumber) || null,
    address:
      identityRaw.address && typeof identityRaw.address === "object"
        ? (identityRaw.address as RhIdentity["address"])
        : { country: "France" },
  };

  const contract: RhContract = {
    type: (["cdi", "cdd", "cddu", "interim", "stage", "autre"] as const).includes(
      contractRaw.type as RhContractType,
    )
      ? (contractRaw.type as RhContractType)
      : null,
    startDate: str(contractRaw.startDate) || null,
    endDate: str(contractRaw.endDate) || null,
    jobTitle: str(contractRaw.jobTitle) || null,
    etablissement: str(contractRaw.etablissement) || null,
    workTimePercent:
      typeof contractRaw.workTimePercent === "number" ? contractRaw.workTimePercent : 100,
    classification: str(contractRaw.classification) || null,
    coefficient: str(contractRaw.coefficient) || null,
    grossMonthlySalary: str(contractRaw.grossMonthlySalary) || null,
  };

  const meta: MetaRhDocument = {
    version: 1,
    id: str(o.id).trim() || rhUid(),
    identity,
    contract,
    banking:
      o.banking && typeof o.banking === "object" ? (o.banking as MetaRhDocument["banking"]) : {},
    emergencyContact:
      o.emergencyContact && typeof o.emergencyContact === "object"
        ? (o.emergencyContact as MetaRhDocument["emergencyContact"])
        : {},
    clerkUserId: str(o.clerkUserId) || null,
    accountStatus: asAccountStatus(o.accountStatus),
    category: asCategory(o.category),
    active: o.active !== false,
    hireDate: str(o.hireDate) || null,
    documents: Array.isArray(o.documents) ? (o.documents as RhDocumentRef[]) : [],
    formations: Array.isArray(o.formations) ? (o.formations as RhFormation[]) : [],
    habilitations: Array.isArray(o.habilitations) ? (o.habilitations as RhHabilitation[]) : [],
    medecineTravail:
      o.medecineTravail && typeof o.medecineTravail === "object"
        ? (o.medecineTravail as RhMedecineTravail)
        : emptyMedecineTravail(),
    entretiens: Array.isArray(o.entretiens) ? (o.entretiens as RhEntretien[]) : [],
    onboarding: (o.onboarding as RhOnboardingState | null) || null,
    complianceFlags: computeRhComplianceFlags({ identity, contract }),
    updatedAt: str(o.updatedAt) || new Date().toISOString(),
    updatedBy: str(o.updatedBy) || undefined,
  };
  return meta;
}

export function normalizeRhPersonnelIndex(raw: unknown, basePath = RH_DEFAULT_BASE_PATH): RhPersonnelIndex {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const entriesRaw = Array.isArray(o.entries) ? o.entries : [];
  const entries: RhPersonnelIndexEntry[] = [];
  for (const row of entriesRaw) {
    if (!row || typeof row !== "object") continue;
    const r = row as Record<string, unknown>;
    const id = String(r.id ?? "").trim();
    const folderName = String(r.folderName ?? "").trim();
    const email = String(r.email ?? "").trim().toLowerCase();
    if (!id || !folderName) continue;
    entries.push({
      id,
      folderName,
      displayName: String(r.displayName ?? folderName).trim(),
      email,
      clerkUserId: typeof r.clerkUserId === "string" ? r.clerkUserId : null,
      category: asCategory(r.category),
      active: r.active !== false,
      accountStatus: asAccountStatus(r.accountStatus),
      hireDate: typeof r.hireDate === "string" ? r.hireDate : null,
    });
  }
  return {
    version: 1,
    updatedAt: typeof o.updatedAt === "string" ? o.updatedAt : new Date().toISOString(),
    basePath: typeof o.basePath === "string" && o.basePath.trim() ? o.basePath.trim() : basePath,
    entries,
  };
}
