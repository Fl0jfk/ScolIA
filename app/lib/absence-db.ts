import "server-only";

import { and, eq, ne } from "drizzle-orm";
import { getDb } from "@/db/index";
import { absence, absenceHistory } from "@/db/schema";
import type { AbsenceRecord } from "@/app/lib/absences-types";
import { normalizeAbsenceRecord } from "@/app/lib/absences-types";
import {
  toAbsenceDateOnly,
  toAbsenceIsoTimestamp,
  toAbsenceIsoTimestampOrNull,
} from "@/app/lib/absence-db-dates";
import {
  isEntCoreDbEnabled,
  resolveCurrentEtablissementId,
} from "@/app/lib/ent-core-db";

function parseTs(raw: string | null | undefined): Date | null {
  if (!raw) return null;
  const d = new Date(raw);
  return Number.isNaN(d.getTime()) ? null : d;
}

export function absenceRecordToRows(etablissementId: string, record: AbsenceRecord) {
  const r = normalizeAbsenceRecord(record);
  const createdAt = parseTs(r.createdAt) ?? new Date();
  const updatedAt = parseTs(r.updatedAt) ?? createdAt;
  let startAt = parseTs(r.data.startAt);
  let endAt = parseTs(r.data.endAt);
  if (!startAt) startAt = parseTs(r.data.startDate) ?? createdAt;
  if (!endAt) endAt = parseTs(r.data.endDate) ?? startAt;
  const main = {
    id: r.id,
    etablissementId,
    createdAt,
    updatedAt,
    source: r.source,
    displayName: r.displayName,
    calendarVisible: r.calendarVisible !== false,
    createdByUserId: r.createdBy?.userId ?? "",
    createdByName: r.createdBy?.name ?? "",
    createdByEmail: r.createdBy?.email ?? "",
    createdByRoles: Array.isArray(r.createdBy?.roles) ? r.createdBy.roles.map(String) : [],
    scope: r.data.scope,
    siteLabel: r.data.etablissement ?? null,
    periodType: r.data.periodType ?? null,
    startDate: toAbsenceDateOnly(r.data.startDate),
    endDate: toAbsenceDateOnly(r.data.endDate),
    startTime: r.data.startTime ?? null,
    endTime: r.data.endTime ?? null,
    startAt,
    endAt,
    reason: r.data.reason ?? "",
    details: r.data.details ?? "",
    sourceDocument: r.data.sourceDocument ?? null,
    documentKeys: Array.isArray(r.data.documentKeys) ? r.data.documentKeys.map(String) : [],
    confidence: typeof r.data.confidence === "number" ? r.data.confidence : null,
    workflowStatus: r.workflowStatus,
    managerDecision: r.managerDecision,
    closedAt: parseTs(r.closedAt ?? null),
    justificationFileName: r.justification?.fileName ?? null,
    justificationFileUrl: r.justification?.fileUrl ?? null,
    justificationUploadedAt: parseTs(r.justification?.uploadedAt ?? null),
    justificationUploadedBy: r.justification?.uploadedBy ?? null,
    managerNote: r.managerNote ?? null,
    hoursTreatment: r.hoursTreatment ? String(r.hoursTreatment) : null,
    justificatifRelanceAt: parseTs(r.justificatifRelanceAt ?? null),
    privacyReasonRedacted: !!r.privacyReasonRedacted,
    privacyDocumentsPurgedAt: parseTs(r.privacyDocumentsPurgedAt ?? null),
    personnelId: r.personnelId?.trim() || null,
    enseignantId: r.enseignantId?.trim() || null,
    adminTreatedAt: parseTs(r.adminTreatedAt ?? null),
    adminTreatedBy: r.adminTreatedBy?.trim() || null,
    adminNote: r.adminNote?.trim() || null,
    staffPreferredTreatment: r.staffPreferredTreatment?.trim() || null,
    staffPreferredMakeupSlots: r.staffPreferredMakeupSlots?.trim() || null,
    directionConfirmedMakeupSlots: r.directionConfirmedMakeupSlots?.trim() || null,
  };
  const history = (r.history ?? []).map((h, i) => ({
    etablissementId,
    absenceId: r.id,
    at: parseTs(h.at) ?? updatedAt,
    by: h.by ?? "",
    action: h.action ?? "",
    note: h.note ?? null,
    sortOrder: i,
  }));
  return { main, history };
}

export function rowsToAbsenceRecord(
  main: typeof absence.$inferSelect,
  historyRows: (typeof absenceHistory.$inferSelect)[],
): AbsenceRecord {
  const sorted = [...historyRows].sort((a, b) => a.sortOrder - b.sortOrder);
  const createdAt = toAbsenceIsoTimestamp(main.createdAt as string | Date);
  const updatedAt = toAbsenceIsoTimestamp(main.updatedAt as string | Date, createdAt);
  return normalizeAbsenceRecord({
    id: main.id,
    createdAt,
    updatedAt,
    source: main.source as AbsenceRecord["source"],
    displayName: main.displayName,
    calendarVisible: main.calendarVisible,
    createdBy: {
      userId: main.createdByUserId,
      name: main.createdByName,
      email: main.createdByEmail,
      roles: main.createdByRoles ?? [],
    },
    data: {
      scope: main.scope as AbsenceRecord["data"]["scope"],
      etablissement: main.siteLabel,
      periodType: main.periodType as AbsenceRecord["data"]["periodType"],
      startDate: toAbsenceDateOnly(main.startDate as string | Date),
      endDate: toAbsenceDateOnly(main.endDate as string | Date),
      startTime: main.startTime,
      endTime: main.endTime,
      startAt: toAbsenceIsoTimestamp(main.startAt as string | Date, createdAt),
      endAt: toAbsenceIsoTimestamp(main.endAt as string | Date, createdAt),
      reason: main.reason,
      details: main.details,
      sourceDocument: main.sourceDocument ?? undefined,
      documentKeys: main.documentKeys ?? [],
      confidence: main.confidence ?? undefined,
    },
    workflowStatus: main.workflowStatus as AbsenceRecord["workflowStatus"],
    managerDecision: main.managerDecision as AbsenceRecord["managerDecision"],
    closedAt: toAbsenceIsoTimestampOrNull(main.closedAt as string | Date | null),
    justification:
      main.justificationFileName || main.justificationFileUrl
        ? {
            fileName: main.justificationFileName ?? "",
            fileUrl: main.justificationFileUrl ?? "",
            uploadedAt:
              toAbsenceIsoTimestampOrNull(
                main.justificationUploadedAt as string | Date | null,
              ) ?? "",
            uploadedBy: main.justificationUploadedBy ?? "",
          }
        : null,
    managerNote: main.managerNote ?? undefined,
    hoursTreatment: (main.hoursTreatment as AbsenceRecord["hoursTreatment"]) ?? null,
    justificatifRelanceAt: toAbsenceIsoTimestampOrNull(
      main.justificatifRelanceAt as string | Date | null,
    ),
    privacyReasonRedacted: main.privacyReasonRedacted,
    privacyDocumentsPurgedAt: toAbsenceIsoTimestampOrNull(
      main.privacyDocumentsPurgedAt as string | Date | null,
    ),
    personnelId: main.personnelId ?? null,
    enseignantId: main.enseignantId ?? null,
    adminTreatedAt: toAbsenceIsoTimestampOrNull(main.adminTreatedAt as string | Date | null),
    adminTreatedBy: main.adminTreatedBy ?? null,
    adminNote: main.adminNote ?? null,
    staffPreferredTreatment: main.staffPreferredTreatment ?? null,
    staffPreferredMakeupSlots: main.staffPreferredMakeupSlots ?? null,
    directionConfirmedMakeupSlots: main.directionConfirmedMakeupSlots ?? null,
    history: sorted.map((h) => ({
      at: toAbsenceIsoTimestamp(h.at as string | Date, updatedAt),
      by: h.by,
      action: h.action,
      ...(h.note ? { note: h.note } : {}),
    })),
  });
}

export async function countAbsencesInDb(etablissementId: string): Promise<number> {
  const db = getDb();
  const rows = await db
    .select({ id: absence.id })
    .from(absence)
    .where(eq(absence.etablissementId, etablissementId));
  return rows.length;
}

export async function listAbsencesFromDb(etablissementId: string): Promise<AbsenceRecord[]> {
  const db = getDb();
  const mains = await db.select().from(absence).where(eq(absence.etablissementId, etablissementId));
  if (mains.length === 0) return [];
  const hist = await db
    .select()
    .from(absenceHistory)
    .where(eq(absenceHistory.etablissementId, etablissementId));
  const byAbsence = new Map<string, (typeof absenceHistory.$inferSelect)[]>();
  for (const h of hist) {
    const list = byAbsence.get(h.absenceId) ?? [];
    list.push(h);
    byAbsence.set(h.absenceId, list);
  }
  const out: AbsenceRecord[] = [];
  for (const m of mains) {
    try {
      out.push(rowsToAbsenceRecord(m, byAbsence.get(m.id) ?? []));
    } catch (e) {
      console.error(`[absence-db] skip row ${m.id}`, e);
    }
  }
  return out;
}

export async function getAbsenceFromDb(
  etablissementId: string,
  id: string,
): Promise<AbsenceRecord | null> {
  const db = getDb();
  const [main] = await db
    .select()
    .from(absence)
    .where(and(eq(absence.etablissementId, etablissementId), eq(absence.id, id)))
    .limit(1);
  if (!main) return null;
  const hist = await db
    .select()
    .from(absenceHistory)
    .where(
      and(eq(absenceHistory.etablissementId, etablissementId), eq(absenceHistory.absenceId, id)),
    );
  return rowsToAbsenceRecord(main, hist);
}

export async function upsertAbsenceInDb(
  etablissementId: string,
  record: AbsenceRecord,
): Promise<void> {
  const db = getDb();
  const { main, history } = absenceRecordToRows(etablissementId, record);
  await db
    .insert(absence)
    .values(main)
    .onConflictDoUpdate({
      target: absence.id,
      set: {
        etablissementId: main.etablissementId,
        createdAt: main.createdAt,
        updatedAt: main.updatedAt,
        source: main.source,
        displayName: main.displayName,
        calendarVisible: main.calendarVisible,
        createdByUserId: main.createdByUserId,
        createdByName: main.createdByName,
        createdByEmail: main.createdByEmail,
        createdByRoles: main.createdByRoles,
        scope: main.scope,
        siteLabel: main.siteLabel,
        periodType: main.periodType,
        startDate: main.startDate,
        endDate: main.endDate,
        startTime: main.startTime,
        endTime: main.endTime,
        startAt: main.startAt,
        endAt: main.endAt,
        reason: main.reason,
        details: main.details,
        sourceDocument: main.sourceDocument,
        documentKeys: main.documentKeys,
        confidence: main.confidence,
        workflowStatus: main.workflowStatus,
        managerDecision: main.managerDecision,
        closedAt: main.closedAt,
        justificationFileName: main.justificationFileName,
        justificationFileUrl: main.justificationFileUrl,
        justificationUploadedAt: main.justificationUploadedAt,
        justificationUploadedBy: main.justificationUploadedBy,
        managerNote: main.managerNote,
        hoursTreatment: main.hoursTreatment,
        justificatifRelanceAt: main.justificatifRelanceAt,
        privacyReasonRedacted: main.privacyReasonRedacted,
        privacyDocumentsPurgedAt: main.privacyDocumentsPurgedAt,
        personnelId: main.personnelId,
        enseignantId: main.enseignantId,
        adminTreatedAt: main.adminTreatedAt,
        adminTreatedBy: main.adminTreatedBy,
        adminNote: main.adminNote,
        staffPreferredTreatment: main.staffPreferredTreatment,
        staffPreferredMakeupSlots: main.staffPreferredMakeupSlots,
        directionConfirmedMakeupSlots: main.directionConfirmedMakeupSlots,
      },
    });
  await db
    .delete(absenceHistory)
    .where(
      and(
        eq(absenceHistory.etablissementId, etablissementId),
        eq(absenceHistory.absenceId, record.id),
      ),
    );
  if (history.length > 0) {
    await db.insert(absenceHistory).values(history);
  }
}

export async function deleteAbsenceFromDb(etablissementId: string, id: string): Promise<void> {
  const db = getDb();
  await db
    .delete(absence)
    .where(and(eq(absence.etablissementId, etablissementId), eq(absence.id, id)));
}

export async function replaceAbsencesInDb(
  etablissementId: string,
  records: AbsenceRecord[],
): Promise<number> {
  const db = getDb();
  // Ne jamais DELETE les absences source=accueil : un replace complet
  // (ingest PDF, wizard, PATCH) en parallèle effaçait les déclarations standard.
  await db
    .delete(absence)
    .where(
      and(eq(absence.etablissementId, etablissementId), ne(absence.source, "accueil")),
    );

  // Upsert unitaire pour tout le monde (évite conflit PK si une ligne survit au DELETE).
  for (const r of records) {
    await upsertAbsenceInDb(etablissementId, r);
  }
  return records.length;
}

export async function absencesDbReady(): Promise<string | null> {
  if (!isEntCoreDbEnabled()) return null;
  return resolveCurrentEtablissementId();
}
