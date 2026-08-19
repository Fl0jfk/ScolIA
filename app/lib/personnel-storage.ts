import { getJson, putJson } from "@/app/lib/s3-storage";
import { computeNextEntretienDue, normalizeMedecineTravail } from "@/app/lib/personnel-rh-cycles";
import {
  PERSONNEL_INDEX_KEY,
  PERSONNEL_SHARED_DOCS_KEY,
  normalizePersonnelRecord,
  personnelRecordKey,
  toIndexEntry,
  type PersonnelIndexEntry,
  type PersonnelRecord,
  type SharedPersonnelDocument,
} from "@/app/lib/personnel-types";

export async function getPersonnelIndex(): Promise<PersonnelIndexEntry[]> {
  const hit = await getJson<PersonnelIndexEntry[]>(PERSONNEL_INDEX_KEY);
  return Array.isArray(hit?.data) ? hit.data : [];
}

async function savePersonnelIndex(index: PersonnelIndexEntry[]) {
  await putJson(PERSONNEL_INDEX_KEY, index);
}

function enrichPersonnelRecord(record: PersonnelRecord): PersonnelRecord {
  const medecineTravail = normalizeMedecineTravail(record.medecineTravail);
  const entretiens = record.entretiens.map((e) => ({
    ...e,
    nextDueAt:
      e.nextDueAt ??
      (e.status === "realise" && e.completedAt ? computeNextEntretienDue(e.completedAt) : null),
  }));
  return { ...record, medecineTravail, entretiens };
}

export async function getPersonnelRecord(id: string): Promise<PersonnelRecord | null> {
  const hit = await getJson<PersonnelRecord>(personnelRecordKey(id));
  return hit?.data ? enrichPersonnelRecord(normalizePersonnelRecord(hit.data)) : null;
}

export async function savePersonnelRecord(record: PersonnelRecord): Promise<PersonnelRecord> {
  const normalized = enrichPersonnelRecord(
    normalizePersonnelRecord({ ...record, updatedAt: new Date().toISOString() }),
  );
  await putJson(personnelRecordKey(normalized.id), normalized);
  const index = await getPersonnelIndex();
  const entry = toIndexEntry(normalized);
  const idx = index.findIndex((e) => e.id === normalized.id);
  if (idx >= 0) index[idx] = entry;
  else index.push(entry);
  await savePersonnelIndex(index);
  return normalized;
}

async function deletePersonnelRecord(id: string) {
  const index = await getPersonnelIndex();
  await savePersonnelIndex(index.filter((e) => e.id !== id));
  // record file left on S3 intentionally (soft) — could delete with deleteObject if needed
}

export async function getSharedPersonnelDocuments(): Promise<SharedPersonnelDocument[]> {
  const hit = await getJson<SharedPersonnelDocument[]>(PERSONNEL_SHARED_DOCS_KEY);
  return Array.isArray(hit?.data) ? hit.data : [];
}

export async function saveSharedPersonnelDocuments(docs: SharedPersonnelDocument[]) {
  await putJson(PERSONNEL_SHARED_DOCS_KEY, docs);
}

export async function getAllPersonnelRecords(): Promise<PersonnelRecord[]> {
  const index = await getPersonnelIndex();
  const records = await Promise.all(index.map((e) => getPersonnelRecord(e.id)));
  return records.filter((r): r is PersonnelRecord => !!r && r.active !== false);
}

export async function findPersonnelByEmail(email: string): Promise<PersonnelRecord | null> {
  const normalized = email.trim().toLowerCase();
  const index = await getPersonnelIndex();
  const hit = index.find((e) => {
    const emails = [e.email, e.emailPerso, e.emailPro]
      .filter(Boolean)
      .map((x) => x!.trim().toLowerCase());
    return emails.includes(normalized);
  });
  if (!hit) return null;
  return getPersonnelRecord(hit.id);
}

export async function findPersonnelByClerkId(clerkUserId: string): Promise<PersonnelRecord | null> {
  const index = await getPersonnelIndex();
  const hit = index.find((e) => e.clerkUserId === clerkUserId);
  if (!hit) return null;
  return getPersonnelRecord(hit.id);
}
