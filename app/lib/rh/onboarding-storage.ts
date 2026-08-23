import "server-only";

import { randomBytes } from "crypto";
import { getJson, putJson } from "@/app/lib/s3-storage";
import {
  RH_ONBOARDING_INDEX_KEY,
  RH_ONBOARDING_TTL_MS,
  onboardingEntryFromRecord,
  normalizeRhOnboardingRecord,
  rhOnboardingRecordKey,
  rhOnboardingTokenKey,
  type RhOnboardingFormData,
  type RhOnboardingIndex,
  type RhOnboardingRecord,
} from "@/app/lib/rh/onboarding-types";
import { rhUid } from "@/app/lib/rh/types";

function emptyIndex(): RhOnboardingIndex {
  return { version: 1, updatedAt: new Date().toISOString(), entries: [] };
}

async function readIndex(): Promise<RhOnboardingIndex> {
  const hit = await getJson<RhOnboardingIndex>(RH_ONBOARDING_INDEX_KEY);
  if (!hit?.data?.entries) return emptyIndex();
  return {
    version: 1,
    updatedAt: hit.data.updatedAt || new Date().toISOString(),
    entries: hit.data.entries,
  };
}

async function writeIndex(index: RhOnboardingIndex): Promise<void> {
  await putJson(RH_ONBOARDING_INDEX_KEY, {
    ...index,
    version: 1,
    updatedAt: new Date().toISOString(),
  });
}

function generateOnboardingToken(): string {
  return randomBytes(24).toString("base64url");
}

export async function getOnboardingRecordById(id: string): Promise<RhOnboardingRecord | null> {
  const hit = await getJson<unknown>(rhOnboardingRecordKey(id));
  return hit?.data ? normalizeRhOnboardingRecord(hit.data) : null;
}

export async function getOnboardingRecordByToken(token: string): Promise<RhOnboardingRecord | null> {
  const trimmed = token.trim();
  if (!trimmed) return null;
  const mapHit = await getJson<{ id?: string }>(rhOnboardingTokenKey(trimmed));
  const id = mapHit?.data?.id?.trim();
  if (!id) return null;
  return getOnboardingRecordById(id);
}

export async function saveOnboardingRecord(record: RhOnboardingRecord): Promise<void> {
  const next = { ...record, updatedAt: new Date().toISOString() };
  await putJson(rhOnboardingRecordKey(next.id), next);
  await putJson(rhOnboardingTokenKey(next.token), { id: next.id, token: next.token });

  const index = await readIndex();
  const entry = onboardingEntryFromRecord(next);
  const idx = index.entries.findIndex((e) => e.id === next.id);
  if (idx >= 0) index.entries[idx] = entry;
  else index.entries.unshift(entry);
  await writeIndex(index);
}

export async function listOnboardingRecords(): Promise<RhOnboardingRecord[]> {
  const index = await readIndex();
  const records: RhOnboardingRecord[] = [];
  for (const entry of index.entries) {
    const r = await getOnboardingRecordById(entry.id);
    if (r) records.push(r);
  }
  return records.sort((a, b) => b.createdAt.localeCompare(a.createdAt));
}

export async function createOnboardingInvite(input: {
  createdBy: { userId: string; name: string; email?: string | null };
  candidateEmailHint?: string | null;
  publicPath: string;
}): Promise<RhOnboardingRecord> {
  const now = new Date();
  const id = rhUid("onb");
  const token = generateOnboardingToken();
  const record: RhOnboardingRecord = {
    id,
    token,
    status: "awaiting_candidate",
    createdAt: now.toISOString(),
    updatedAt: now.toISOString(),
    expiresAt: new Date(now.getTime() + RH_ONBOARDING_TTL_MS).toISOString(),
    createdBy: input.createdBy,
    candidateEmailHint: input.candidateEmailHint?.trim().toLowerCase() || null,
    publicPath: input.publicPath,
    form: null,
    invitePending: false,
  };
  await saveOnboardingRecord(record);
  return record;
}

export async function submitOnboardingForm(
  token: string,
  form: RhOnboardingFormData,
): Promise<RhOnboardingRecord> {
  const record = await getOnboardingRecordByToken(token);
  if (!record) throw new Error("Lien invalide ou expiré.");
  if (record.status === "cancelled") throw new Error("Ce parcours a été annulé.");
  if (new Date(record.expiresAt).getTime() < Date.now()) {
    throw new Error("Ce lien a expiré. Contactez la RH pour en obtenir un nouveau.");
  }
  if (record.status !== "awaiting_candidate" && record.status !== "submitted") {
    throw new Error("Ce formulaire a déjà été traité.");
  }

  const next: RhOnboardingRecord = {
    ...record,
    form,
    status: "submitted",
    submittedAt: new Date().toISOString(),
  };
  await saveOnboardingRecord(next);
  return next;
}
