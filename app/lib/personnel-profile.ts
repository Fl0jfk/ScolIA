import type { PersonnelRecord } from "@/app/lib/personnel-types";

/** Données administratives réutilisables (contrat, Sécu, prévoyance, etc.). */
export type PersonnelProfile = {
  birthDate?: string | null;
  birthPlace?: string | null;
  birthName?: string | null;
  nationality?: string | null;
  gender?: "M" | "F" | "autre" | null;
  socialSecurityNumber?: string | null;
  addressLine1?: string | null;
  addressLine2?: string | null;
  postalCode?: string | null;
  city?: string | null;
  country?: string | null;
  phone?: string | null;
  phoneMobile?: string | null;
  maritalStatus?: string | null;
  childrenCount?: number | null;
  iban?: string | null;
  bic?: string | null;
  internalId?: string | null;
  contractType?: "cdi" | "cdd" | "cddu" | "interim" | "stage" | "autre" | null;
  contractEndDate?: string | null;
  workTimePercent?: number | null;
  classification?: string | null;
  coefficient?: string | null;
  grossMonthlySalary?: string | null;
  establishment?: string | null;
  emergencyContactName?: string | null;
  emergencyContactPhone?: string | null;
  notes?: string | null;
};

export function defaultPersonnelProfile(): PersonnelProfile {
  return {
    birthDate: null,
    birthPlace: null,
    birthName: null,
    nationality: "Française",
    gender: null,
    socialSecurityNumber: null,
    addressLine1: null,
    addressLine2: null,
    postalCode: null,
    city: null,
    country: "France",
    phone: null,
    phoneMobile: null,
    maritalStatus: null,
    childrenCount: null,
    iban: null,
    bic: null,
    internalId: null,
    contractType: null,
    contractEndDate: null,
    workTimePercent: 100,
    classification: null,
    coefficient: null,
    grossMonthlySalary: null,
    establishment: null,
    emergencyContactName: null,
    emergencyContactPhone: null,
    notes: null,
  };
}

function str(v: unknown): string | null {
  if (v === null || v === undefined) return null;
  const s = String(v).trim();
  return s || null;
}

function num(v: unknown): number | null {
  if (v === null || v === undefined || v === "") return null;
  const n = Number(v);
  return Number.isFinite(n) ? n : null;
}

const CONTRACT_TYPES = new Set(["cdi", "cdd", "cddu", "interim", "stage", "autre"]);
const GENDERS = new Set(["M", "F", "autre"]);

export function normalizePersonnelProfile(raw: unknown): PersonnelProfile {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const base = defaultPersonnelProfile();
  const contractRaw = str(o.contractType);
  const genderRaw = str(o.gender);

  return {
    ...base,
    birthDate: str(o.birthDate),
    birthPlace: str(o.birthPlace),
    birthName: str(o.birthName),
    nationality: str(o.nationality) ?? base.nationality,
    gender: genderRaw && GENDERS.has(genderRaw) ? (genderRaw as PersonnelProfile["gender"]) : null,
    socialSecurityNumber: str(o.socialSecurityNumber),
    addressLine1: str(o.addressLine1),
    addressLine2: str(o.addressLine2),
    postalCode: str(o.postalCode),
    city: str(o.city),
    country: str(o.country) ?? base.country,
    phone: str(o.phone),
    phoneMobile: str(o.phoneMobile),
    maritalStatus: str(o.maritalStatus),
    childrenCount: num(o.childrenCount),
    iban: str(o.iban),
    bic: str(o.bic),
    internalId: str(o.internalId),
    contractType:
      contractRaw && CONTRACT_TYPES.has(contractRaw)
        ? (contractRaw as PersonnelProfile["contractType"])
        : null,
    contractEndDate: str(o.contractEndDate),
    workTimePercent: num(o.workTimePercent) ?? base.workTimePercent,
    classification: str(o.classification),
    coefficient: str(o.coefficient),
    grossMonthlySalary: str(o.grossMonthlySalary),
    establishment: str(o.establishment),
    emergencyContactName: str(o.emergencyContactName),
    emergencyContactPhone: str(o.emergencyContactPhone),
    notes: str(o.notes),
  };
}

export function profileFromFormData(fd: FormData): PersonnelProfile {
  return normalizePersonnelProfile({
    birthDate: fd.get("birthDate"),
    birthPlace: fd.get("birthPlace"),
    birthName: fd.get("birthName"),
    nationality: fd.get("nationality"),
    gender: fd.get("gender"),
    socialSecurityNumber: fd.get("socialSecurityNumber"),
    addressLine1: fd.get("addressLine1"),
    addressLine2: fd.get("addressLine2"),
    postalCode: fd.get("postalCode"),
    city: fd.get("city"),
    country: fd.get("country"),
    phone: fd.get("phone"),
    phoneMobile: fd.get("phoneMobile"),
    maritalStatus: fd.get("maritalStatus"),
    childrenCount: fd.get("childrenCount"),
    iban: fd.get("iban"),
    bic: fd.get("bic"),
    internalId: fd.get("internalId"),
    contractType: fd.get("contractType"),
    contractEndDate: fd.get("contractEndDate"),
    workTimePercent: fd.get("workTimePercent"),
    classification: fd.get("classification"),
    coefficient: fd.get("coefficient"),
    grossMonthlySalary: fd.get("grossMonthlySalary"),
    establishment: fd.get("establishment"),
    emergencyContactName: fd.get("emergencyContactName"),
    emergencyContactPhone: fd.get("emergencyContactPhone"),
    notes: fd.get("notes"),
  });
}

function profileToJsonBody(profile: PersonnelProfile): Record<string, unknown> {
  return { ...profile };
}
