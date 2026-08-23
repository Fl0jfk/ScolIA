import "server-only";

import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  teacherPlanning,
  teacherPlanningReplacement,
  teacherPlanningSlot,
} from "@/db/schema";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  emptyTeacherPlanning,
  normalizeTeacherPlanning,
  type TeacherPlanningDoc,
  type TeacherPlanningSlot,
  type TeacherReplacementSlot,
} from "@/app/lib/rh/planning-types";

function asWeekType(v: string): "A" | "B" | null {
  return v === "A" || v === "B" ? v : null;
}

function rowToSlot(row: {
  id: string;
  day: number;
  startTime: string;
  endTime: string;
  subject: string;
  classes: string[] | null;
  room: string | null;
}): TeacherPlanningSlot {
  return {
    id: row.id,
    day: row.day as 1 | 2 | 3 | 4 | 5,
    start: row.startTime,
    end: row.endTime,
    subject: row.subject,
    classes: row.classes ?? [],
    room: row.room || undefined,
  };
}

function rowToReplacement(row: {
  id: string;
  date: string;
  startTime: string;
  endTime: string;
  subject: string;
  classes: string[] | null;
  room: string | null;
  note: string | null;
  createdBy: string;
  createdAt: Date;
}): TeacherReplacementSlot {
  return {
    id: row.id,
    date: row.date,
    start: row.startTime,
    end: row.endTime,
    subject: row.subject,
    classes: row.classes ?? [],
    room: row.room || undefined,
    note: row.note || undefined,
    createdBy: row.createdBy,
    createdAt: row.createdAt.toISOString(),
  };
}

export async function readTeacherPlanningFromDb(
  etablissementId: string,
  externalUserId: string,
): Promise<TeacherPlanningDoc | null> {
  const db = getDb();
  const [meta] = await db
    .select()
    .from(teacherPlanning)
    .where(
      and(
        eq(teacherPlanning.etablissementId, etablissementId),
        eq(teacherPlanning.externalUserId, externalUserId),
      ),
    )
    .limit(1);
  if (!meta) return null;

  const slots = await db
    .select()
    .from(teacherPlanningSlot)
    .where(eq(teacherPlanningSlot.planningId, meta.id));
  const replacements = await db
    .select()
    .from(teacherPlanningReplacement)
    .where(eq(teacherPlanningReplacement.planningId, meta.id));

  const weekA = slots.filter((s) => s.weekType === "A").map(rowToSlot);
  const weekB = slots.filter((s) => s.weekType === "B").map(rowToSlot);

  return normalizeTeacherPlanning(
    {
      version: 1,
      kind: "teacher",
      personnelId: externalUserId,
      weekA,
      weekB,
      replacements: replacements.map(rowToReplacement),
      updatedAt: meta.updatedAt.toISOString(),
      updatedBy: meta.updatedBy,
      source:
        meta.source === "pdf_import" || meta.source === "manual" ? meta.source : undefined,
      sourceFileName: meta.sourceFileName || undefined,
    },
    externalUserId,
  );
}

export async function writeTeacherPlanningToDb(
  etablissementId: string,
  doc: TeacherPlanningDoc,
): Promise<TeacherPlanningDoc> {
  const normalized = normalizeTeacherPlanning(doc, doc.personnelId);
  const db = getDb();

  const existing = await db
    .select({ id: teacherPlanning.id })
    .from(teacherPlanning)
    .where(
      and(
        eq(teacherPlanning.etablissementId, etablissementId),
        eq(teacherPlanning.externalUserId, normalized.personnelId),
      ),
    )
    .limit(1);

  let planningId = existing[0]?.id;
  const updatedAt = new Date(normalized.updatedAt || Date.now());

  if (planningId) {
    await db
      .update(teacherPlanning)
      .set({
        source: normalized.source ?? null,
        sourceFileName: normalized.sourceFileName ?? null,
        updatedAt,
        updatedBy: normalized.updatedBy || "",
      })
      .where(eq(teacherPlanning.id, planningId));
    await db
      .delete(teacherPlanningSlot)
      .where(eq(teacherPlanningSlot.planningId, planningId));
    await db
      .delete(teacherPlanningReplacement)
      .where(eq(teacherPlanningReplacement.planningId, planningId));
  } else {
    const [inserted] = await db
      .insert(teacherPlanning)
      .values({
        etablissementId,
        externalUserId: normalized.personnelId,
        source: normalized.source ?? null,
        sourceFileName: normalized.sourceFileName ?? null,
        updatedAt,
        updatedBy: normalized.updatedBy || "",
      })
      .returning({ id: teacherPlanning.id });
    planningId = inserted!.id;
  }

  const slotRows = [
    ...normalized.weekA.map((s) => ({
      id: s.id,
      etablissementId,
      planningId: planningId!,
      weekType: "A",
      day: s.day,
      startTime: s.start,
      endTime: s.end,
      subject: s.subject,
      classes: s.classes,
      room: s.room ?? null,
    })),
    ...normalized.weekB.map((s) => ({
      id: s.id,
      etablissementId,
      planningId: planningId!,
      weekType: "B",
      day: s.day,
      startTime: s.start,
      endTime: s.end,
      subject: s.subject,
      classes: s.classes,
      room: s.room ?? null,
    })),
  ];
  if (slotRows.length) {
    await db.insert(teacherPlanningSlot).values(slotRows);
  }

  const replRows = normalized.replacements.map((r) => ({
    id: r.id,
    etablissementId,
    planningId: planningId!,
    date: r.date,
    startTime: r.start,
    endTime: r.end,
    subject: r.subject,
    classes: r.classes,
    room: r.room ?? null,
    note: r.note ?? null,
    createdBy: r.createdBy || "",
    createdAt: new Date(r.createdAt || Date.now()),
  }));
  if (replRows.length) {
    await db.insert(teacherPlanningReplacement).values(replRows);
  }

  return (
    (await readTeacherPlanningFromDb(etablissementId, normalized.personnelId)) ||
    emptyTeacherPlanning(normalized.personnelId)
  );
}

/** Liste tous les planning profs relationnels d’un établissement (index EDT). */
export async function listTeacherPlanningsFromDb(
  etablissementId: string,
): Promise<TeacherPlanningDoc[]> {
  const db = getDb();
  const metas = await db
    .select()
    .from(teacherPlanning)
    .where(eq(teacherPlanning.etablissementId, etablissementId));
  const out: TeacherPlanningDoc[] = [];
  for (const meta of metas) {
    const doc = await readTeacherPlanningFromDb(etablissementId, meta.externalUserId);
    if (doc && (doc.weekA.length || doc.weekB.length || doc.replacements.length)) {
      out.push(doc);
    }
  }
  return out;
}

export async function resolveTeacherPlanningEtabId(): Promise<string | null> {
  return resolveCurrentEtablissementId();
}

export { asWeekType };
