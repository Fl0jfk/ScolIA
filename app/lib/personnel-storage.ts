import { getJson, putJson } from "@/app/lib/s3-storage";
import {
  countPersonnelInDb,
  getPersonnelFromDb,
  isEntCoreDbEnabled,
  listPersonnelFromDb,
  resolveCurrentEtablissementId,
  upsertPersonnelInDb,
} from "@/app/lib/ent-core-db";
import { computeNextEntretienDue, normalizeMedecineTravail } from "@/app/lib/personnel-rh-cycles";
import {
  PERSONNEL_SHARED_DOCS_KEY,
  normalizePersonnelRecord,
  toIndexEntry,
  type PersonnelIndexEntry,
  type PersonnelRecord,
  type SharedPersonnelDocument,
} from "@/app/lib/personnel-types";

export async function getPersonnelIndex(): Promise<PersonnelIndexEntry[]> {
  if (!isEntCoreDbEnabled()) return [];
  try {
    const etabId = await resolveCurrentEtablissementId();
    if (!etabId) return [];
    const records = await listPersonnelFromDb(etabId);
    return records.map(toIndexEntry);
  } catch (error) {
    console.error("[personnel-storage] index DB", error);
    return [];
  }
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
  if (!isEntCoreDbEnabled()) return null;
  try {
    const etabId = await resolveCurrentEtablissementId();
    if (!etabId) return null;
    const fromDb = await getPersonnelFromDb(etabId, id);
    return fromDb ? enrichPersonnelRecord(fromDb) : null;
  } catch (error) {
    console.error("[personnel-storage] lecture DB", error);
    return null;
  }
}

export async function savePersonnelRecord(record: PersonnelRecord): Promise<PersonnelRecord> {
  const normalized = enrichPersonnelRecord(
    normalizePersonnelRecord({ ...record, updatedAt: new Date().toISOString() }),
  );
  if (!isEntCoreDbEnabled()) throw new Error("[personnel] Postgres requis");
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) throw new Error("[personnel] établissement introuvable");
  await upsertPersonnelInDb(etabId, normalized);
  return normalized;
}

export async function getSharedPersonnelDocuments(): Promise<SharedPersonnelDocument[]> {
  const hit = await getJson<SharedPersonnelDocument[]>(PERSONNEL_SHARED_DOCS_KEY);
  return Array.isArray(hit?.data) ? hit.data : [];
}

export async function saveSharedPersonnelDocuments(docs: SharedPersonnelDocument[]) {
  await putJson(PERSONNEL_SHARED_DOCS_KEY, docs);
}

export async function getAllPersonnelRecords(): Promise<PersonnelRecord[]> {
  if (!isEntCoreDbEnabled()) return [];
  try {
    const etabId = await resolveCurrentEtablissementId();
    if (!etabId) return [];
    if ((await countPersonnelInDb(etabId)) === 0) return [];
    return (await listPersonnelFromDb(etabId)).map(enrichPersonnelRecord);
  } catch (error) {
    console.error("[personnel-storage] lecture DB", error);
    return [];
  }
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

export async function findPersonnelByExternalId(externalUserId: string): Promise<PersonnelRecord | null> {
  const index = await getPersonnelIndex();
  const hit = index.find((e) => e.externalUserId === externalUserId);
  if (!hit) return null;
  return getPersonnelRecord(hit.id);
}
