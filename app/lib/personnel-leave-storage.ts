import "server-only";

import { and, desc, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { personnelLeave } from "@/db/schema";
import {
  isEntCoreDbEnabled,
  resolveCurrentEtablissementId,
} from "@/app/lib/ent-core-db";
import type { PersonnelLeaveRequest } from "@/app/lib/personnel-types";

function requireDb(): void {
  if (!isEntCoreDbEnabled()) {
    throw new Error("[personnel-leave] Postgres requis — plus de JSON");
  }
}

function rowToLeave(row: typeof personnelLeave.$inferSelect): PersonnelLeaveRequest {
  return {
    id: row.id,
    personnelId: row.personnelId,
    personnelName: row.personnelName,
    type: row.type as PersonnelLeaveRequest["type"],
    startDate: row.startDate,
    endDate: row.endDate,
    ...(row.reason ? { reason: row.reason } : {}),
    status: row.status as PersonnelLeaveRequest["status"],
    createdAt: row.createdAt,
    ...(row.decidedAt ? { decidedAt: row.decidedAt } : {}),
    ...(row.decidedBy ? { decidedBy: row.decidedBy } : {}),
    ...(row.decisionNote ? { decisionNote: row.decisionNote } : {}),
  };
}

export async function getPersonnelLeaveRequests(): Promise<PersonnelLeaveRequest[]> {
  requireDb();
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return [];
  const db = getDb();
  const rows = await db
    .select()
    .from(personnelLeave)
    .where(eq(personnelLeave.etablissementId, etabId))
    .orderBy(desc(personnelLeave.createdAt));
  return rows.map(rowToLeave);
}

export async function upsertPersonnelLeaveRequest(request: PersonnelLeaveRequest) {
  requireDb();
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) throw new Error("[personnel-leave] établissement introuvable");
  const db = getDb();
  const values = {
    id: request.id,
    etablissementId: etabId,
    personnelId: request.personnelId,
    personnelName: request.personnelName || "",
    type: request.type,
    startDate: request.startDate,
    endDate: request.endDate,
    reason: request.reason ?? null,
    status: request.status,
    createdAt: request.createdAt,
    decidedAt: request.decidedAt ?? null,
    decidedBy: request.decidedBy ?? null,
    decisionNote: request.decisionNote ?? null,
  };
  await db
    .insert(personnelLeave)
    .values(values)
    .onConflictDoUpdate({
      target: personnelLeave.id,
      set: {
        etablissementId: values.etablissementId,
        personnelId: values.personnelId,
        personnelName: values.personnelName,
        type: values.type,
        startDate: values.startDate,
        endDate: values.endDate,
        reason: values.reason,
        status: values.status,
        createdAt: values.createdAt,
        decidedAt: values.decidedAt,
        decidedBy: values.decidedBy,
        decisionNote: values.decisionNote,
      },
    });
  return request;
}

export async function getPersonnelLeaveRequestById(
  id: string,
): Promise<PersonnelLeaveRequest | null> {
  requireDb();
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return null;
  const db = getDb();
  const [row] = await db
    .select()
    .from(personnelLeave)
    .where(and(eq(personnelLeave.etablissementId, etabId), eq(personnelLeave.id, id)))
    .limit(1);
  return row ? rowToLeave(row) : null;
}
