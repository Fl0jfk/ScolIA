import "server-only";

import { randomBytes } from "crypto";
import { getJson, putJson } from "@/app/lib/s3-storage";
import type { MicrosoftLicensePerson, MicrosoftLicenseRequest } from "@/app/lib/microsoft-license-types";
import { MAX_A3_LICENSES } from "@/app/lib/microsoft-license-types";

const MICROSOFT_LICENSE_STORAGE_KEY = "microsoft-license-request.json";

export async function loadMicrosoftLicenseRequest(): Promise<MicrosoftLicenseRequest | null> {
  const hit = await getJson<MicrosoftLicenseRequest>(MICROSOFT_LICENSE_STORAGE_KEY);
  return hit?.data ?? null;
}

type SaveMicrosoftLicenseInput = {
  people: Omit<MicrosoftLicensePerson, "id">[];
  notes?: string;
  submittedBy?: string;
  establishmentName?: string;
};

export function validateMicrosoftLicensePeople(
  people: Omit<MicrosoftLicensePerson, "id">[],
): string | null {
  if (!people.length) return "Ajoutez au moins une personne.";
  const a3Count = people.filter((p) => p.licenseType === "A3").length;
  if (a3Count > MAX_A3_LICENSES) {
    return `Maximum ${MAX_A3_LICENSES} licences A3.`;
  }
  for (const p of people) {
    if (!p.firstName.trim() || !p.lastName.trim()) return "Nom et prénom obligatoires.";
    if (!p.email.trim() || !p.email.includes("@")) return "E-mail invalide.";
    if (!p.jobRole.trim()) return "Rôle / fonction obligatoire.";
  }
  return null;
}

export async function saveMicrosoftLicenseRequest(
  input: SaveMicrosoftLicenseInput,
): Promise<MicrosoftLicenseRequest> {
  const err = validateMicrosoftLicensePeople(input.people);
  if (err) throw new Error(err);

  const payload: MicrosoftLicenseRequest = {
    version: 1,
    submittedAt: new Date().toISOString(),
    submittedBy: input.submittedBy,
    establishmentName: input.establishmentName,
    notes: input.notes?.trim() || undefined,
    people: input.people.map((p) => ({
      id: randomBytes(6).toString("hex"),
      firstName: p.firstName.trim(),
      lastName: p.lastName.trim(),
      email: p.email.trim().toLowerCase(),
      jobRole: p.jobRole.trim(),
      licenseType: p.licenseType,
    })),
  };

  await putJson(MICROSOFT_LICENSE_STORAGE_KEY, payload);
  return payload;
}
