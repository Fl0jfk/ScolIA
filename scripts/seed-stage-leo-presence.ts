/**
 * Seed convention stage signée pour Leo JUSTIF (Value Gate présence stages).
 * Idempotent : id fixe `stg_seed_leo_presence`.
 *
 * Usage :
 *   NODE_TLS_REJECT_UNAUTHORIZED=0 node --require ./scripts/stub-server-only.cjs --import tsx scripts/seed-stage-leo-presence.ts
 */
import { existsSync, readFileSync } from "node:fs";
import { eq } from "drizzle-orm";
import { closeDb, getDb, isDatabaseConfigured } from "../db/index";
import { etablissement } from "../db/schema";
import {
  getCollectionRecord,
  upsertCollectionRecord,
} from "../app/lib/ent-collection-db";
import { jsonPathToCollection } from "../app/lib/ent-json-postgres";
import {
  currentStageSchoolYear,
  STAGE_S3,
  type StageConvention,
  type StageConventionIndexEntry,
} from "../app/lib/stage-types";

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

const TENANT_SLUG = "default";
const ELEVE_ID = "872a645d-1db9-4f1c-98cf-9d7a89ed81d8";
const CONVENTION_ID = "stg_seed_leo_presence";

function parisDateOffset(daysFromToday: number): string {
  const today = new Date().toLocaleDateString("en-CA", { timeZone: "Europe/Paris" });
  const d = new Date(`${today}T12:00:00+02:00`);
  d.setDate(d.getDate() + daysFromToday);
  return d.toLocaleDateString("en-CA", { timeZone: "Europe/Paris" });
}

async function main() {
  if (!isDatabaseConfigured()) throw new Error("DATABASE_URL manquant");
  process.env.ENT_CORE_DB = process.env.ENT_CORE_DB || "1";

  const db = getDb();
  const [etab] = await db
    .select()
    .from(etablissement)
    .where(eq(etablissement.slug, TENANT_SLUG))
    .limit(1);
  if (!etab) throw new Error(`Établissement ${TENANT_SLUG} introuvable`);

  const periodStart = parisDateOffset(-2);
  const periodEnd = parisDateOffset(5);
  const today = parisDateOffset(0);
  const now = new Date().toISOString();

  const convention: StageConvention = {
    id: CONVENTION_ID,
    schoolYear: currentStageSchoolYear(),
    status: "signed",
    internshipKind: "pfmp",
    student: {
      firstName: "Leo",
      lastName: "JUSTIF",
      className: "4B",
      level: "4e",
    },
    company: {
      name: "Acme Stage Seed",
      address: "1 rue du Stage",
      postalCode: "76000",
      city: "Rouen",
      activity: "Informatique",
      tutorName: "Alice Tuteur",
      tutorEmail: "tuteur@acme.seed",
    },
    schedule: {
      mode: "uniform_week",
      periodStart,
      periodEnd,
      days: [
        {
          weekday: 1,
          hasLunchBreak: true,
          morningStart: "09:00",
          morningEnd: "12:00",
          afternoonStart: "13:30",
          afternoonEnd: "17:00",
        },
      ],
      presenceWeekdays: [1, 2, 3, 4, 5],
    },
    teacherReferent: { name: "Prof Seed", email: "prof@localhost.dev" },
    signatures: [],
    createdAt: now,
    updatedAt: now,
    createdBy: { role: "staff", name: "seed-stage-leo-presence" },
    history: [{ at: now, by: "seed", action: "seed_presence_value_gate" }],
    eleveDossierFiling: {
      filedAt: now,
      filedBy: "seed",
      eleveId: ELEVE_ID,
      documentId: "doc_seed_stage_leo",
      s3Key: "stages/seed/leo.pdf",
      title: "Convention seed Leo",
    },
  };

  const convPath = STAGE_S3.convention(CONVENTION_ID);
  const { collection: convCol, recordId: convId } = jsonPathToCollection(convPath);
  await upsertCollectionRecord(etab.id, convCol, convId, {
    ...(convention as unknown as Record<string, unknown>),
    id: CONVENTION_ID,
  });

  const indexPath = STAGE_S3.conventionsIndex;
  const { collection: idxCol, recordId: idxId } = jsonPathToCollection(indexPath);
  const existing = await getCollectionRecord<{ __root?: StageConventionIndexEntry[] } & Record<string, unknown>>(
    etab.id,
    idxCol,
    idxId,
  );
  let index: StageConventionIndexEntry[] = [];
  if (existing) {
    if (Array.isArray((existing as { __root?: unknown }).__root)) {
      index = (existing as { __root: StageConventionIndexEntry[] }).__root;
    } else if (Array.isArray(existing)) {
      index = existing as unknown as StageConventionIndexEntry[];
    }
  }

  const entry: StageConventionIndexEntry = {
    id: CONVENTION_ID,
    status: convention.status,
    studentName: `${convention.student.firstName} ${convention.student.lastName}`.trim(),
    className: convention.student.className,
    level: convention.student.level,
    companyName: convention.company.name,
    internshipKind: convention.internshipKind,
    periodStart,
    periodEnd,
    schoolYear: convention.schoolYear,
    updatedAt: now,
    teacherReferentEmail: convention.teacherReferent.email.toLowerCase(),
  };
  const pos = index.findIndex((x) => x.id === CONVENTION_ID);
  if (pos >= 0) index[pos] = entry;
  else index.unshift(entry);

  await upsertCollectionRecord(etab.id, idxCol, idxId, { __root: index });

  console.log(
    JSON.stringify(
      {
        ok: true,
        etablissementId: etab.id,
        conventionId: CONVENTION_ID,
        eleveId: ELEVE_ID,
        periodStart,
        periodEnd,
        today,
      },
      null,
      2,
    ),
  );
  await closeDb();
}

main().catch(async (e) => {
  console.error(e);
  try {
    await closeDb();
  } catch {
    /* ignore */
  }
  process.exit(1);
});
