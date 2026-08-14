import { getAbsenceDocumentKeys } from "@/app/lib/absences-documents";
import { absencePersonNamesMatch } from "@/app/lib/absences-shared-utils";
import {
  normalizeAbsenceRecord,
  resolveAbsenceScope,
  type AbsenceRecord,
  type AbsenceScope,
  type Etablissement,
} from "@/app/lib/absences-types";
import { resolveTravelsS3ObjectKey } from "@/app/lib/travels-s3";

export {isPendingAbsenceRecord} from "@/app/lib/absences-shared-utils";

type AbsenceDuplicateCandidate = {
  displayName: string;
  scope: AbsenceScope;
  etablissement: Etablissement | null;
  startAt: string;
  endAt: string;
};

export function absenceCandidateFromRecord(record: AbsenceRecord): AbsenceDuplicateCandidate {
  return {
    displayName: record.displayName || record.createdBy.name,
    scope: resolveAbsenceScope(record),
    etablissement: record.data.etablissement,
    startAt: record.data.startAt,
    endAt: record.data.endAt,
  };
}

function absencePeriodsOverlap(
  aStart: string,
  aEnd: string,
  bStart: string,
  bEnd: string,
): boolean {
  const sa = +new Date(aStart);
  const ea = +new Date(aEnd);
  const sb = +new Date(bStart);
  const eb = +new Date(bEnd);
  if ([sa, ea, sb, eb].some(Number.isNaN)) return false;
  return ea >= sb && eb >= sa;
}

function isDuplicateAbsence(
  existing: AbsenceRecord,
  candidate: AbsenceDuplicateCandidate,
): boolean {
  const existingScope = resolveAbsenceScope(existing);
  if (existingScope !== candidate.scope) return false;
  if (
    candidate.scope === "professeur" &&
    existing.data.etablissement &&
    candidate.etablissement &&
    existing.data.etablissement !== candidate.etablissement
  ) {
    return false;
  }
  if (
    !absencePersonNamesMatch(
      existing.displayName || existing.createdBy.name,
      candidate.displayName,
    )
  ) {
    return false;
  }
  return absencePeriodsOverlap(
    existing.data.startAt,
    existing.data.endAt,
    candidate.startAt,
    candidate.endAt,
  );
}

export function findDuplicateAbsence(
  index: AbsenceRecord[],
  candidate: AbsenceDuplicateCandidate,
  excludeId?: string,
): AbsenceRecord | null {
  for (const record of index) {
    if (excludeId && record.id === excludeId) continue;
    if (isDuplicateAbsence(record, candidate)) return record;
  }
  return null;
}

function mergeTextBlocks(...parts: Array<string | null | undefined>): string {
  const unique: string[] = [];
  for (const raw of parts) {
    const text = String(raw || "").trim();
    if (!text) continue;
    if (!unique.some((item) => item.toLowerCase() === text.toLowerCase())) unique.push(text);
  }
  return unique.join("\n");
}

function collectDocumentKeysSync(record: AbsenceRecord): string[] {
  return getAbsenceDocumentKeys(record);
}

export function mergeAbsenceRecordsSync(
  existing: AbsenceRecord,
  incoming: AbsenceRecord,
  actor: string,
): AbsenceRecord {
  const now = new Date().toISOString();
  const startMs = Math.min(+new Date(existing.data.startAt), +new Date(incoming.data.startAt));
  const endMs = Math.max(+new Date(existing.data.endAt), +new Date(incoming.data.endAt));
  const mergedStart = new Date(startMs).toISOString();
  const mergedEnd = new Date(endMs).toISOString();

  const docKeys = new Set<string>([
    ...collectDocumentKeysSync(existing),
    ...collectDocumentKeysSync(incoming),
  ]);

  let justification = existing.justification;
  if (incoming.justification?.fileUrl) {
    if (!justification?.fileUrl) {
      justification = incoming.justification;
    } else if (justification.fileUrl !== incoming.justification.fileUrl) {
      const extraName = incoming.justification.fileName || "justificatif";
      const note = `Justificatif complémentaire : ${extraName}`;
      incoming = {
        ...incoming,
        data: {
          ...incoming.data,
          details: mergeTextBlocks(incoming.data.details, note),
        },
      };
    }
  }

  const displayName =
    (existing.displayName || existing.createdBy.name).length >=
    (incoming.displayName || incoming.createdBy.name).length
      ? existing.displayName || existing.createdBy.name
      : incoming.displayName || incoming.createdBy.name;

  return normalizeAbsenceRecord({
    ...existing,
    updatedAt: now,
    displayName,
    calendarVisible: existing.calendarVisible || incoming.calendarVisible,
    managerDecision:
      existing.managerDecision === "VALIDEE" || incoming.managerDecision === "VALIDEE"
        ? "VALIDEE"
        : existing.managerDecision,
    workflowStatus:
      existing.workflowStatus === "CLOTUREE" || incoming.workflowStatus === "CLOTUREE"
        ? "CLOTUREE"
        : existing.workflowStatus,
    data: {
      ...existing.data,
      startAt: mergedStart,
      endAt: mergedEnd,
      startDate: mergedStart.slice(0, 10),
      endDate: mergedEnd.slice(0, 10),
      reason: mergeTextBlocks(existing.data.reason, incoming.data.reason),
      details: mergeTextBlocks(existing.data.details, incoming.data.details),
      documentKeys: [...docKeys],
      sourceDocument: mergeTextBlocks(existing.data.sourceDocument, incoming.data.sourceDocument),
      confidence: Math.max(existing.data.confidence ?? 0, incoming.data.confidence ?? 0),
    },
    justification,
    history: [
      ...(existing.history || []),
      {
        at: now,
        by: actor,
        action: "FUSION_DOUBLON",
        note: `Fusion automatique avec l'absence ${incoming.id}`,
      },
    ],
  });
}

async function mergeAbsenceRecords(
  existing: AbsenceRecord,
  incoming: AbsenceRecord,
  actor: string,
): Promise<AbsenceRecord> {
  const docKeys = new Set<string>([
    ...collectDocumentKeysSync(existing),
    ...collectDocumentKeysSync(incoming),
  ]);

  for (const record of [existing, incoming]) {
    const justificationUrl = record.justification?.fileUrl?.trim();
    if (justificationUrl) {
      const key = await resolveTravelsS3ObjectKey(justificationUrl);
      if (key) docKeys.add(key);
    }
  }

  const merged = mergeAbsenceRecordsSync(existing, incoming, actor);
  return normalizeAbsenceRecord({
    ...merged,
    data: { ...merged.data, documentKeys: [...docKeys] },
  });
}

/** Fusionne les doublons dans une liste (version rapide, sans appels S3). */
export function consolidateDuplicateAbsencesSync(
  index: AbsenceRecord[],
): { index: AbsenceRecord[]; removedIds: string[]; updatedIds: string[] } {
  const sorted = [...index].sort((a, b) => +new Date(a.createdAt) - +new Date(b.createdAt));
  const kept: AbsenceRecord[] = [];
  const removedIds: string[] = [];
  const updatedIds: string[] = [];

  for (const record of sorted) {
    const duplicate = findDuplicateAbsence(kept, absenceCandidateFromRecord(record));
    if (duplicate) {
      const pos = kept.findIndex((item) => item.id === duplicate.id);
      kept[pos] = mergeAbsenceRecordsSync(duplicate, record, "Système");
      updatedIds.push(duplicate.id);
      removedIds.push(record.id);
      continue;
    }
    kept.push(record);
  }

  return { index: kept, removedIds, updatedIds };
}
