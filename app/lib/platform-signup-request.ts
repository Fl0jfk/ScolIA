import "server-only";

import { randomBytes } from "crypto";
import { getPlatformJson, isPlatformStorageWritable, putPlatformJson } from "@/app/lib/platform-storage";
import type {
  CreateSignupRequestInput,
  TenantSignupAdminContact,
  TenantSignupEstablishment,
  TenantSignupPostalAddress,
  TenantSignupRequest,
  TenantSignupStatus,
  TenantSignupAuditEntry,
} from "@/app/lib/platform-signup-types";

export type {CreateSignupRequestInput, TenantSignupRequest} from "@/app/lib/platform-signup-types";
export {slugifyEstablishmentName} from "@/app/lib/platform-signup-types";

type TenantSignupIndexEntry = {
  id: string;
  status: TenantSignupStatus;
  legalName: string;
  rne: string;
  adminEmail: string;
  createdAt: string;
  updatedAt: string;
};

type TenantSignupIndex = {
  version: 1;
  requests: TenantSignupIndexEntry[];
};

const SIGNUP_REQUEST_PREFIX = "platform/signup-requests";
const SIGNUP_INDEX_KEY = `${SIGNUP_REQUEST_PREFIX}/index.json`;

const RNE_PATTERN = /^[0-9]{7}[A-Z]$/i;

function normalizeRne(value: string): string {
  return value.trim().toUpperCase().replace(/\s/g, "");
}

function isValidRne(value: string): boolean {
  return RNE_PATTERN.test(normalizeRne(value));
}

function requestKey(id: string): string {
  return `${SIGNUP_REQUEST_PREFIX}/${id}.json`;
}

function newId(): string {
  return randomBytes(12).toString("hex");
}

function newAccessToken(): string {
  return randomBytes(24).toString("base64url");
}

function indexEntryFromRequest(req: TenantSignupRequest): TenantSignupIndexEntry {
  return {
    id: req.id,
    status: req.status,
    legalName: req.establishment.legalName,
    rne: req.establishment.rne,
    adminEmail: req.adminContact.email,
    createdAt: req.createdAt,
    updatedAt: req.updatedAt,
  };
}

async function loadIndex(): Promise<TenantSignupIndex> {
  const hit = await getPlatformJson<TenantSignupIndex>(SIGNUP_INDEX_KEY);
  if (hit?.version === 1 && Array.isArray(hit.requests)) return hit;
  return { version: 1, requests: [] };
}

async function saveIndex(index: TenantSignupIndex): Promise<void> {
  await putPlatformJson(SIGNUP_INDEX_KEY, index);
}

export async function loadSignupRequest(id: string): Promise<TenantSignupRequest | null> {
  return getPlatformJson<TenantSignupRequest>(requestKey(id));
}

export async function loadSignupRequestByToken(token: string): Promise<TenantSignupRequest | null> {
  const t = token.trim();
  if (!t) return null;
  const index = await loadIndex();
  for (const entry of index.requests) {
    const req = await loadSignupRequest(entry.id);
    if (req?.accessToken === t) return req;
  }
  return null;
}

export async function listSignupRequests(): Promise<TenantSignupRequest[]> {
  const index = await loadIndex();
  const sorted = [...index.requests].sort(
    (a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime(),
  );
  const out: TenantSignupRequest[] = [];
  for (const entry of sorted) {
    const req = await loadSignupRequest(entry.id);
    if (req) out.push(req);
  }
  return out;
}

function validateCreateSignupInput(input: CreateSignupRequestInput): string | null {
  const name = input.establishment.legalName?.trim();
  if (!name || name.length < 2) return "Nom de l'établissement requis.";
  const rne = normalizeRne(input.establishment.rne || "");
  if (!isValidRne(rne)) return "RNE / UAI invalide (format attendu : 7 chiffres + 1 lettre).";
  const street = input.establishment.postalAddress?.street?.trim();
  const zip = input.establishment.postalAddress?.zip?.trim();
  const city = input.establishment.postalAddress?.city?.trim();
  if (!street || !zip || !city) return "Adresse postale complète requise.";
  const count = Number(input.establishment.estimatedStudentCount);
  if (!Number.isFinite(count) || count < 1) return "Effectif élèves estimé invalide.";
  if (
    ![
      "internal_establishment",
      "external_provider",
      "none",
    ].includes(input.establishment.microsoftCurrentManagement)
  ) {
    return "Mode de gestion Microsoft actuel invalide.";
  }
  if (
    !["scola_takeover", "scola_independent"].includes(
      input.establishment.microsoftTargetMode,
    )
  ) {
    return "Mode Microsoft souhaité invalide.";
  }
  if (
    input.establishment.microsoftCurrentManagement === "external_provider" &&
    input.establishment.microsoftTargetMode === "scola_takeover"
  ) {
    return "Avec un prestataire externe, choisissez un déploiement Microsoft Scola indépendant.";
  }
  const email = input.adminContact.email?.trim().toLowerCase();
  if (!email || !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) return "E-mail administrateur invalide.";
  const firstName = input.adminContact.firstName?.trim();
  const lastName = input.adminContact.lastName?.trim();
  if (!firstName || !lastName) return "Prénom et nom du référent requis.";
  const msContact = input.establishment.microsoftDecisionContact;
  if (!msContact?.fullName?.trim()) {
    return "Nom du contact décisionnaire Microsoft requis.";
  }
  if (!msContact?.email?.trim() || !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(msContact.email)) {
    return "E-mail du contact décisionnaire Microsoft invalide.";
  }
  return null;
}

export async function createSignupRequest(input: CreateSignupRequestInput): Promise<TenantSignupRequest> {
  if (!isPlatformStorageWritable()) {
    throw new Error("Stockage plateforme indisponible.");
  }
  const err = validateCreateSignupInput(input);
  if (err) throw new Error(err);
  const microsoftTargetMode =
    input.establishment.microsoftCurrentManagement === "external_provider"
      ? "scola_independent"
      : input.establishment.microsoftTargetMode;

  const now = new Date().toISOString();
  const request: TenantSignupRequest = {
    id: newId(),
    accessToken: newAccessToken(),
    status: "pending_microsoft",
    createdAt: now,
    updatedAt: now,
    establishment: {
      legalName: input.establishment.legalName.trim(),
      rne: normalizeRne(input.establishment.rne),
      kind: input.establishment.kind === "standalone" ? "standalone" : "groupe",
      postalAddress: {
        street: input.establishment.postalAddress.street.trim(),
        zip: input.establishment.postalAddress.zip.trim(),
        city: input.establishment.postalAddress.city.trim(),
      },
      estimatedStudentCount: Math.round(Number(input.establishment.estimatedStudentCount)),
      wantsMicrosoftLicenses: Boolean(input.establishment.wantsMicrosoftLicenses),
      microsoftCurrentManagement: input.establishment.microsoftCurrentManagement,
      microsoftTargetMode,
      microsoftDecisionContact: {
        fullName: input.establishment.microsoftDecisionContact?.fullName?.trim() || "",
        ...(input.establishment.microsoftDecisionContact?.role?.trim()
          ? { role: input.establishment.microsoftDecisionContact.role.trim() }
          : {}),
        email:
          input.establishment.microsoftDecisionContact?.email?.trim().toLowerCase() || "",
        ...(input.establishment.microsoftDecisionContact?.phone?.trim()
          ? { phone: input.establishment.microsoftDecisionContact.phone.trim() }
          : {}),
      },
    },
    adminContact: {
      firstName: input.adminContact.firstName.trim(),
      lastName: input.adminContact.lastName.trim(),
      email: input.adminContact.email.trim().toLowerCase(),
      ...(input.adminContact.phone?.trim() ? { phone: input.adminContact.phone.trim() } : {}),
      ...(input.adminContact.jobTitle?.trim() ? { jobTitle: input.adminContact.jobTitle.trim() } : {}),
    },
    auditLog: [{ at: now, action: "submitted", detail: "Dossier reçu via formulaire public" }],
  };

  await putPlatformJson(requestKey(request.id), request);
  const index = await loadIndex();
  index.requests = index.requests.filter((r) => r.id !== request.id);
  index.requests.unshift(indexEntryFromRequest(request));
  await saveIndex(index);
  return request;
}

export async function saveSignupRequest(
  request: TenantSignupRequest,
  audit?: Omit<TenantSignupAuditEntry, "at">,
): Promise<TenantSignupRequest> {
  const now = new Date().toISOString();
  const next: TenantSignupRequest = {
    ...request,
    updatedAt: now,
    auditLog: audit
      ? [...(request.auditLog || []), { at: now, ...audit }]
      : request.auditLog || [],
  };
  await putPlatformJson(requestKey(next.id), next);
  const index = await loadIndex();
  const idx = index.requests.findIndex((r) => r.id === next.id);
  const entry = indexEntryFromRequest(next);
  if (idx >= 0) index.requests[idx] = entry;
  else index.requests.unshift(entry);
  await saveIndex(index);
  return next;
}

export function publicSignupStatusView(request: TenantSignupRequest) {
  return {
    id: request.id,
    status: request.status,
    legalName: request.establishment.legalName,
    rne: request.establishment.rne,
    createdAt: request.createdAt,
    updatedAt: request.updatedAt,
    rejectedReason: request.status === "rejected" ? request.rejectedReason : undefined,
    canPay: request.status === "microsoft_approved" || request.status === "pending_payment",
    provisionedTenantSlug: request.provisionedTenantSlug,
  };
}
