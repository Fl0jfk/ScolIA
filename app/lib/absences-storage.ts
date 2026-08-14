import { DeleteObjectCommand } from "@aws-sdk/client-s3";
import { getAbsenceDocumentKeys, isDocumentKeyReferenced } from "@/app/lib/absences-documents";
import {
  absenceCandidateFromRecord,
  consolidateDuplicateAbsencesSync,
  findDuplicateAbsence,
  isPendingAbsenceRecord,
  mergeAbsenceRecordsSync,
} from "@/app/lib/absences-dedup";
import { isDocumentKeyReferencedInLegacy } from "@/app/lib/absences-legacy-convocations";
import { isSensitiveAbsenceContent } from "@/app/lib/absences-privacy";
import {
  ABSENCES_INDEX_KEY,
  normalizeAbsenceRecord,
  resolveAbsenceScope,
  type AbsenceRecord,
} from "@/app/lib/absences-types";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName, getJson, putJson } from "@/app/lib/s3-storage";
import { s3Key } from "@/app/lib/s3-path";
import { resolveTravelsS3ObjectKey } from "@/app/lib/travels-s3";

function recordKey(id: string) {
  return `absences/${id}.json`;
}

export async function getAbsenceIndex(): Promise<AbsenceRecord[]> {
  const hit = await getJson<AbsenceRecord[]>(ABSENCES_INDEX_KEY);
  return (hit?.data ?? []).map(normalizeAbsenceRecord);
}

export async function saveAbsenceIndex(index: AbsenceRecord[]) {
  await putJson(ABSENCES_INDEX_KEY, index);
}

export async function getAbsenceRecord(id: string): Promise<AbsenceRecord | null> {
  const hit = await getJson<AbsenceRecord>(recordKey(id));
  return hit?.data ? normalizeAbsenceRecord(hit.data) : null;
}

export async function saveAbsenceRecord(record: AbsenceRecord) {
  const normalized = normalizeAbsenceRecord(record);
  await putJson(recordKey(normalized.id), normalized);
  return normalized;
}

async function deleteAbsenceRecordJson(id: string) {
  const bucket = await getBucketName();
  const s3Client = await getTenantDataS3Client();
  try {
    await s3Client.send(
      new DeleteObjectCommand({ Bucket: bucket, Key: s3Key(recordKey(id)) }),
    );
  } catch {
    /* déjà supprimé */
  }
}

/** Enregistre ou fusionne si une absence équivalente existe déjà. */
export async function saveOrMergeAbsenceRecord(
  index: AbsenceRecord[],
  incoming: AbsenceRecord,
  actor: string,
): Promise<{ index: AbsenceRecord[]; record: AbsenceRecord; merged: boolean; removedId?: string }> {
  const duplicate = findDuplicateAbsence(index, absenceCandidateFromRecord(incoming), incoming.id);
  if (!duplicate) {
    const saved = await saveAbsenceRecord(incoming);
    const next = index.filter((item) => item.id !== saved.id);
    next.push(saved);
    return { index: next, record: saved, merged: false };
  }

  const merged = mergeAbsenceRecordsSync(duplicate, incoming, actor);
  await saveAbsenceRecord(merged);
  await deleteAbsenceRecordJson(incoming.id);

  const next = index.filter((item) => item.id !== incoming.id && item.id !== duplicate.id);
  next.push(merged);
  return { index: next, record: merged, merged: true, removedId: incoming.id };
}

/** Fusionne uniquement les absences en cours (à traiter), sans toucher au calendrier historique. */
export async function consolidatePendingAbsencesInIndex(
  fullIndex: AbsenceRecord[],
): Promise<AbsenceRecord[]> {
  const pending = fullIndex.filter(isPendingAbsenceRecord);
  if (pending.length < 2) return fullIndex;

  const consolidated = consolidateDuplicateAbsencesSync(pending);
  if (consolidated.removedIds.length === 0) return fullIndex;

  for (const id of consolidated.removedIds) {
    await deleteAbsenceRecordJson(id);
  }

  const updated = new Set(consolidated.updatedIds);
  for (const record of consolidated.index) {
    if (updated.has(record.id)) await saveAbsenceRecord(record);
  }

  const removed = new Set(consolidated.removedIds);
  const others = fullIndex.filter((record) => !isPendingAbsenceRecord(record) && !removed.has(record.id));
  const nextIndex = [...others, ...consolidated.index];
  await saveAbsenceIndex(nextIndex);
  return nextIndex;
}

/** Conservé pour compatibilité : on ne supprime plus les absences (seules les pièces sensibles le sont). */
export async function purgeExpiredAbsences(index: AbsenceRecord[]) {
  return index;
}

async function upsertAbsenceInIndex(record: AbsenceRecord) {
  const index = await getAbsenceIndex();
  const pos = index.findIndex((a) => a.id === record.id);
  if (pos >= 0) index[pos] = record;
  else index.push(record);
  await saveAbsenceIndex(index);
  return index;
}

async function deleteStorageKeyIfUnused(key: string, index: AbsenceRecord[]) {
  const trimmed = key.trim();
  if (!trimmed) return;
  const stillUsed =
    isDocumentKeyReferenced(index, trimmed) || (await isDocumentKeyReferencedInLegacy(trimmed));
  if (stillUsed) return;

  const bucket = await getBucketName();
  const s3Client = await getTenantDataS3Client();
  await s3Client.send(new DeleteObjectCommand({ Bucket: bucket, Key: s3Key(trimmed) }));
}

/** Supprime les fichiers S3 liés à une absence (sans supprimer l'absence elle-même). */
async function purgeAbsenceDocumentsFromStorage(
  record: AbsenceRecord,
  index: AbsenceRecord[],
): Promise<AbsenceRecord> {
  const keys = new Set<string>(getAbsenceDocumentKeys(record));
  const justificationUrl = record.justification?.fileUrl?.trim();
  if (justificationUrl) {
    const justificationKey = await resolveTravelsS3ObjectKey(justificationUrl);
    if (justificationKey) keys.add(justificationKey);
  }

  const indexWithoutCurrent = index.filter((item) => item.id !== record.id);
  for (const key of keys) {
    try {
      await deleteStorageKeyIfUnused(key, indexWithoutCurrent);
    } catch (error) {
      console.error(`Absences privacy purge error (${record.id}, ${key}):`, error);
    }
  }

  const now = new Date().toISOString();
  return {
    ...record,
    updatedAt: now,
    justification: null,
    data: {
      ...record.data,
      details: "",
      sourceDocument: undefined,
      documentKeys: undefined,
    },
    privacyDocumentsPurgedAt: now,
    history: [
      ...(record.history || []),
      {
        at: now,
        by: "Système",
        action: "RGPD_PIECES_SUPPRIMEES",
        note: "Pièces jointes sensibles supprimées après envoi du mail de validation.",
      },
    ],
  };
}

/** Après validation : conserve l'absence, purge les pièces sensibles, anonymise le motif si besoin. */
export async function applyPostValidationPrivacy(
  record: AbsenceRecord,
  index: AbsenceRecord[],
): Promise<AbsenceRecord> {
  const scope = resolveAbsenceScope(record);
  const sensitive = isSensitiveAbsenceContent(
    record.data.reason,
    record.data.details,
    record.justification?.fileName,
  );
  const shouldRedactReason = scope === "ogec" || sensitive;
  const shouldPurgeDocs = sensitive;

  if (!shouldRedactReason && !shouldPurgeDocs) return record;

  let next = record;
  if (shouldPurgeDocs) {
    next = await purgeAbsenceDocumentsFromStorage(next, index);
  }

  if (!shouldRedactReason) return next;

  const now = new Date().toISOString();
  return {
    ...next,
    data: {
      ...next.data,
      reason: "Absence",
      details: "",
    },
    privacyReasonRedacted: true,
    updatedAt: now,
    history: [
      ...(next.history || []),
      {
        at: now,
        by: "Système",
        action: "RGPD_MOTIF_ANONYMISE",
        note: scope === "ogec" ? "Motif masqué (personnel OGEC)." : "Motif masqué (donnée sensible).",
      },
    ],
  };
}
