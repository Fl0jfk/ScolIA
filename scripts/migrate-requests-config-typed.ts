/**
 * Migre settings/requests-routing.json et settings/requests-org.json
 * depuis ent_collection → tables request_routing_* / request_org_*.
 *
 * Usage: npm run ent:migrate-requests-config -- --tenant=default
 */
import { existsSync, readFileSync } from "node:fs";
import { eq } from "drizzle-orm";
import { getCollectionRecord } from "../app/lib/ent-collection-db";
import { migrateRequestsConfigEnvelopeToDb } from "../app/lib/requests-config-db";
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

const PATHS = ["settings/requests-routing.json", "settings/requests-org.json"] as const;

async function main() {
  if (!isDatabaseConfigured()) throw new Error("DATABASE_URL manquant");
  process.env.ENT_CORE_DB = process.env.ENT_CORE_DB || "1";

  const slug = argValue("tenant");
  if (!slug) throw new Error("Usage: --tenant=<slug>");

  const db = getDb();
  const [row] = await db.select().from(etablissement).where(eq(etablissement.slug, slug)).limit(1);
  if (!row) throw new Error(`Établissement inconnu: ${slug}`);

  console.log(`Migration config demandes | ${slug} | ${row.id}`);

  for (const path of PATHS) {
    const collection = "settings";
    const recordId = path.replace("settings/", "").replace(".json", "");
    const raw = await getCollectionRecord<Record<string, unknown>>(row.id, collection, recordId);
    if (!raw) {
      console.log(`  skip ${path} (absent en collection)`);
      continue;
    }
    const envelope = "__root" in raw ? raw.__root : raw;
    const ok = await migrateRequestsConfigEnvelopeToDb(row.id, path, envelope);
    console.log(`  ${ok ? "OK" : "skip"} ${path}`);
  }

  await closeDb();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
