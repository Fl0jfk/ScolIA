import { getJson } from "@/app/lib/s3-storage";
import { getDocumentKeys } from "@/app/lib/absences-documents";
import { normalizeAbsenceRecord, type AbsenceRecord } from "@/app/lib/absences-types";

const CONVOCATIONS_INDEX_KEY = "convocations/index.json";

type LegacyConvocationRecord = {
  id: string;
  createdAt?: string;
  updatedAt?: string;
  data: {
    etablissement?: string;
    startDate?: string;
    endDate?: string;
    teacherName?: string;
    examType?: string;
    startAt: string;
    endAt: string;
    sourceDocument?: string;
    documentKey?: string;
    documentKeys?: string[];
    confidence?: number;
  };
};

const LEGACY_CREATED_BY: AbsenceRecord["createdBy"] = {
  userId: "legacy-convocations",
  name: "Import convocation",
  email: "",
  roles: ["administratif"],
};

function isValidLegacyConvocation(record: unknown): record is LegacyConvocationRecord {
  if (!record || typeof record !== "object") return false;
  const r = record as LegacyConvocationRecord;
  if (!r.id || !r.data?.startAt || !r.data?.endAt) return false;
  return true;
}

function convertLegacyConvocation(record: LegacyConvocationRecord): AbsenceRecord {
  const data = record.data || ({} as LegacyConvocationRecord["data"]);
  const sourceDocument = String(data.sourceDocument || "");
  const isManual = /saisie manuelle/i.test(sourceDocument);
  const source = isManual ? "admin_manual" : "admin_pdf";
  const now = record.updatedAt || record.createdAt || new Date().toISOString();
  const documentKeys = getDocumentKeys(data);

  const raw: AbsenceRecord = {
    id: record.id,
    createdAt: record.createdAt || now,
    updatedAt: record.updatedAt || now,
    source,
    displayName: String(data.teacherName || "").trim() || "Inconnu",
    calendarVisible: true,
    createdBy: LEGACY_CREATED_BY,
    data: {
      scope: "professeur",
      etablissement: (data.etablissement as AbsenceRecord["data"]["etablissement"]) || "Collège",
      periodType: data.startDate === data.endDate ? "single_day" : "multi_day",
      startDate: data.startDate || data.startAt?.slice(0, 10) || "",
      endDate: data.endDate || data.endAt?.slice(0, 10) || "",
      startTime: null,
      endTime: null,
      startAt: data.startAt,
      endAt: data.endAt,
      reason: String(data.examType || "").trim() || "Absence",
      details: "",
      sourceDocument,
      documentKeys,
      confidence: data.confidence ?? 1,
    },
    workflowStatus: "CLOTUREE",
    managerDecision: "VALIDEE",
    closedAt: now,
    justification: null,
    justificatifRelanceAt: null,
    history: [
      {
        at: now,
        by: LEGACY_CREATED_BY.name,
        action: "LEGACY_CONVOCATION",
      },
    ],
  };

  return normalizeAbsenceRecord(raw);
}

async function getLegacyConvocationIndex(): Promise<AbsenceRecord[]> {
  const hit = await getJson<LegacyConvocationRecord[]>(CONVOCATIONS_INDEX_KEY);
  const out: AbsenceRecord[] = [];
  for (const entry of hit?.data ?? []) {
    if (!isValidLegacyConvocation(entry)) continue;
    try {
      out.push(convertLegacyConvocation(entry));
    } catch (e) {
      console.warn("[absences-legacy] entrée ignorée:", entry?.id, e);
    }
  }
  return out;
}

/** Fusionne les convocations historiques (S3) pour le calendrier si pas encore dans absences/. */
export async function mergeLegacyConvocationsForCalendar(index: AbsenceRecord[]): Promise<AbsenceRecord[]> {
  try {
    const legacy = await getLegacyConvocationIndex();
    if (legacy.length === 0) return index;

    const known = new Set(index.map((r) => r.id));
    const merged = [...index];
    for (const rec of legacy) {
      if (!known.has(rec.id)) merged.push(rec);
    }
    return merged;
  } catch (e) {
    console.error("[absences-legacy] fusion calendrier échouée:", e);
    return index;
  }
}

export async function getAbsenceOrLegacyRecord(id: string): Promise<AbsenceRecord | null> {
  const hit = await getJson<AbsenceRecord>(`absences/${id}.json`);
  if (hit?.data) return normalizeAbsenceRecord(hit.data);

  const legacyHit = await getJson<LegacyConvocationRecord>(`convocations/${id}.json`);
  if (legacyHit?.data) return convertLegacyConvocation(legacyHit.data);

  return null;
}

export async function deleteLegacyConvocation(id: string) {
  const { DeleteObjectCommand } = await import("@aws-sdk/client-s3");
  const { getBucketName, getJson, putJson } = await import("@/app/lib/s3-storage");
  const { getTenantDataS3Client } = await import("@/app/lib/s3-clients");
  const { s3Key } = await import("@/app/lib/s3-path");

  const bucket = await getBucketName();
  const client = await getTenantDataS3Client();

  await client.send(
    new DeleteObjectCommand({
      Bucket: bucket,
      Key: s3Key(`convocations/${id}.json`),
    }),
  );

  const convHit = await getJson<LegacyConvocationRecord[]>(CONVOCATIONS_INDEX_KEY);
  const convIndex = convHit?.data ?? [];
  if (convIndex.length > 0) {
    await putJson(
      CONVOCATIONS_INDEX_KEY,
      convIndex.filter((r) => r.id !== id),
    );
  }
}

export async function isDocumentKeyReferencedInLegacy(key: string) {
  const legacy = await getLegacyConvocationIndex();
  return legacy.some((r) => getDocumentKeys(r.data).includes(key.trim()));
}
