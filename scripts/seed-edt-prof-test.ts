/**
 * Seed local P2 : affecte Enseignant Test à 3°A + EDT type (3°A + 5°B).
 *
 * Usage:
 *   NODE_TLS_REJECT_UNAUTHORIZED=0 node --require ./scripts/stub-server-only.cjs --import tsx scripts/seed-edt-prof-test.ts
 */
import { existsSync, readFileSync } from "node:fs";
import { and, eq, sql } from "drizzle-orm";
import { closeDb, getDb, isDatabaseConfigured } from "../db/index";
import {
  etablissement,
  schoolClassAssignment,
  schoolRosterMeta,
  user,
} from "../db/schema";
import { upsertCollectionRecord } from "../app/lib/ent-collection-db";
import { jsonPathToCollection } from "../app/lib/ent-json-postgres";
import { emptyTeacherPlanning, type TeacherPlanningDoc } from "../app/lib/rh/planning-types";

function loadEnvFile(path: string) {
  if (!existsSync(path)) return;
  for (const line of readFileSync(path, "utf8").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const i = trimmed.indexOf("=");
    if (i < 0) continue;
    const key = trimmed.slice(0, i).trim();
    let val = trimmed.slice(i + 1).trim();
    if (
      (val.startsWith('"') && val.endsWith('"')) ||
      (val.startsWith("'") && val.endsWith("'"))
    ) {
      val = val.slice(1, -1);
    }
    if (!(key in process.env)) process.env[key] = val;
  }
}

loadEnvFile(".env.local");
loadEnvFile(".env");

const TENANT_SLUG = "la-providence-nicolas-barre";
const TEACHER_EMAIL = "enseignant.test@scolia.local";
const PP_CLASS = "3°A";
const SUBJECT_CLASS = "5°B";

function classKey(className: string): string {
  return className.trim().toLowerCase();
}

function slotId(prefix: string, n: number) {
  return `${prefix}_${n}`;
}

function buildTeacherEdt(personnelId: string): TeacherPlanningDoc {
  const base = emptyTeacherPlanning(personnelId, "seed-edt-prof-test");
  const weekA = [
    {
      id: slotId("a", 1),
      day: 1 as const,
      start: "08:00",
      end: "09:00",
      subject: "MATHS",
      classes: [PP_CLASS],
      room: "Salle 12",
    },
    {
      id: slotId("a", 2),
      day: 1 as const,
      start: "09:00",
      end: "10:00",
      subject: "MATHS",
      classes: [PP_CLASS],
      room: "Salle 12",
    },
    {
      id: slotId("a", 3),
      day: 2 as const,
      start: "10:00",
      end: "11:00",
      subject: "MATHS",
      classes: [SUBJECT_CLASS],
      room: "Salle 14",
    },
    {
      id: slotId("a", 4),
      day: 3 as const,
      start: "14:00",
      end: "15:00",
      subject: "MATHS",
      classes: [PP_CLASS],
      room: "Salle 12",
    },
    {
      id: slotId("a", 5),
      day: 4 as const,
      start: "08:00",
      end: "09:00",
      subject: "MATHS",
      classes: [SUBJECT_CLASS],
      room: "Salle 14",
    },
    {
      id: slotId("a", 6),
      day: 5 as const,
      start: "11:00",
      end: "12:00",
      subject: "MATHS",
      classes: [PP_CLASS],
      room: "Salle 12",
    },
  ];
  const weekB = weekA.map((s, i) => ({
    ...s,
    id: slotId("b", i + 1),
    room: s.room === "Salle 12" ? "Salle 12B" : "Salle 14B",
  }));

  return {
    ...base,
    weekA,
    weekB,
    replacements: [],
    source: "manual",
    updatedAt: new Date().toISOString(),
    updatedBy: "seed-edt-prof-test",
  };
}

async function main() {
  if (!isDatabaseConfigured()) throw new Error("DATABASE_URL manquant");
  process.env.ENT_CORE_DB = process.env.ENT_CORE_DB || "1";
  process.env.NODE_TLS_REJECT_UNAUTHORIZED = "0";

  const db = getDb();
  const [etab] = await db
    .select()
    .from(etablissement)
    .where(eq(etablissement.slug, TENANT_SLUG))
    .limit(1);
  if (!etab) throw new Error(`Établissement ${TENANT_SLUG} introuvable`);

  const [teacher] = await db
    .select()
    .from(user)
    .where(and(eq(user.email, TEACHER_EMAIL), eq(user.etablissementId, etab.id)))
    .limit(1);
  if (!teacher) throw new Error(`Utilisateur ${TEACHER_EMAIL} introuvable`);

  const externalUserId = teacher.externalUserId?.trim() || teacher.id;
  const displayName = teacher.name || "Enseignant Test";

  console.log("[seed] etab", etab.id, etab.slug);
  console.log("[seed] teacher", teacher.email, "businessId=", externalUserId);

  const [meta] = await db
    .select()
    .from(schoolRosterMeta)
    .where(eq(schoolRosterMeta.etablissementId, etab.id))
    .limit(1);
  const catalog = [...new Set([...(meta?.teacherCatalog ?? []), displayName])];
  await db
    .insert(schoolRosterMeta)
    .values({
      etablissementId: etab.id,
      teacherCatalog: catalog,
      updatedAt: new Date(),
      updatedBy: "seed-edt-prof-test",
    })
    .onConflictDoUpdate({
      target: schoolRosterMeta.etablissementId,
      set: {
        teacherCatalog: catalog,
        updatedAt: new Date(),
        updatedBy: "seed-edt-prof-test",
      },
    });

  const existingAssign = await db
    .select()
    .from(schoolClassAssignment)
    .where(
      and(
        eq(schoolClassAssignment.etablissementId, etab.id),
        eq(schoolClassAssignment.classKey, classKey(PP_CLASS)),
      ),
    )
    .limit(1);

  if (existingAssign[0]) {
    await db
      .update(schoolClassAssignment)
      .set({
        className: PP_CLASS,
        externalUserId,
        name: displayName,
        email: teacher.email.toLowerCase(),
        updatedAt: new Date(),
      })
      .where(eq(schoolClassAssignment.id, existingAssign[0].id));
    console.log("[seed] roster updated", PP_CLASS, "→", displayName);
  } else {
    await db.insert(schoolClassAssignment).values({
      etablissementId: etab.id,
      className: PP_CLASS,
      classKey: classKey(PP_CLASS),
      externalUserId,
      name: displayName,
      email: teacher.email.toLowerCase(),
    });
    console.log("[seed] roster inserted", PP_CLASS, "→", displayName);
  }

  const planning = buildTeacherEdt(externalUserId);
  const relativePath = `rh/planning/teachers/${externalUserId}.json`;
  const { collection, recordId } = jsonPathToCollection(relativePath);
  await upsertCollectionRecord(etab.id, collection, recordId, {
    ...planning,
    id: recordId,
  });
  console.log("[seed] EDT written", relativePath, {
    weekA: planning.weekA.length,
    weekB: planning.weekB.length,
    classes: [PP_CLASS, SUBJECT_CLASS],
  });

  const elevesPp = await db.execute<{ n: number }>(sql`
    SELECT count(*)::int AS n FROM eleve
    WHERE etablissement_id = ${etab.id}::uuid
      AND status = 'inscrit'
      AND lower(classe) = ${classKey(PP_CLASS)}
  `);
  const elevesSub = await db.execute<{ n: number }>(sql`
    SELECT count(*)::int AS n FROM eleve
    WHERE etablissement_id = ${etab.id}::uuid
      AND status = 'inscrit'
      AND lower(classe) = ${classKey(SUBJECT_CLASS)}
  `);
  const rowsPp = Array.isArray(elevesPp) ? elevesPp : [];
  const rowsSub = Array.isArray(elevesSub) ? elevesSub : [];
  console.log("[seed] élèves", PP_CLASS, rowsPp[0]?.n ?? "?", "|", SUBJECT_CLASS, rowsSub[0]?.n ?? "?");
  console.log("[seed] OK — reconnecte Enseignant Test puis /eleves/dossiers et /mon-planning");
  await closeDb();
}

main().catch(async (e) => {
  console.error(e);
  await closeDb().catch(() => undefined);
  process.exit(1);
});
