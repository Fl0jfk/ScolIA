/**
 * Migre les EDT profs (collection JSON rh__planning__teachers) → tables relationnelles.
 *
 * Usage:
 *   NODE_TLS_REJECT_UNAUTHORIZED=0 npm run migrate:teacher-planning -- --tenant=la-providence-nicolas-barre
 */
import { existsSync, readFileSync } from "node:fs";
import { eq } from "drizzle-orm";
import { listCollectionRecords } from "../app/lib/ent-collection-db";
import { writeTeacherPlanningToDb } from "../app/lib/rh/planning-teacher-db";
import { normalizeTeacherPlanning } from "../app/lib/rh/planning-types";
import { closeDb, getDb, isDatabaseConfigured } from "../db/index";
import { etablissement } from "../db/schema";

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

function argValue(name: string): string | null {
  const prefix = `--${name}=`;
  const hit = process.argv.find((a) => a.startsWith(prefix));
  return hit ? hit.slice(prefix.length).trim() : null;
}

async function main() {
  if (!isDatabaseConfigured()) throw new Error("DATABASE_URL manquant");
  process.env.ENT_CORE_DB = process.env.ENT_CORE_DB || "1";
  process.env.NODE_TLS_REJECT_UNAUTHORIZED = "0";

  const slug = argValue("tenant");
  if (!slug) throw new Error("Usage: --tenant=<slug>");

  const db = getDb();
  const [etab] = await db
    .select()
    .from(etablissement)
    .where(eq(etablissement.slug, slug))
    .limit(1);
  if (!etab) throw new Error(`Établissement ${slug} introuvable`);

  const records = await listCollectionRecords<Record<string, unknown>>(
    etab.id,
    "rh__planning__teachers",
  );
  console.log(`[migrate] ${records.length} document(s) collection rh__planning__teachers`);

  let ok = 0;
  let skip = 0;
  for (const raw of records) {
    const personnelId = String(raw.personnelId || raw.id || "").trim();
    if (!personnelId) {
      skip += 1;
      continue;
    }
    const doc = normalizeTeacherPlanning(raw, personnelId);
    if (!doc.weekA.length && !doc.weekB.length && !doc.replacements.length) {
      skip += 1;
      continue;
    }
    await writeTeacherPlanningToDb(etab.id, doc);
    ok += 1;
    console.log(
      `  ✓ ${personnelId} — A:${doc.weekA.length} B:${doc.weekB.length} repl:${doc.replacements.length}`,
    );
  }

  console.log(`[migrate] OK ${ok} · ignorés ${skip}`);
  await closeDb();
}

main().catch(async (e) => {
  console.error(e);
  await closeDb().catch(() => undefined);
  process.exit(1);
});
