/**
 * Backfill idempotent : ent_collection stages → tables typées stage_*.
 *
 * Usage :
 *   npx tsx --env-file=.env.local --require ./scripts/stub-server-only.cjs scripts/migrate-stages-to-typed.mjs
 *   npx tsx --env-file=.env.local --require ./scripts/stub-server-only.cjs scripts/migrate-stages-to-typed.mjs --tenant=la-providence-nicolas-barre
 *   npx tsx ... scripts/migrate-stages-to-typed.mjs --dry-run
 *
 * Ne supprime rien dans ent_collection. Upsert unitaire uniquement.
 */
import { existsSync, readFileSync } from "node:fs";
import { eq } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "../db/index.ts";
import { etablissement } from "../db/schema.ts";

function loadEnvFile(path) {
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
process.env.ENT_CORE_DB = process.env.ENT_CORE_DB || "1";

function argValue(name) {
  const prefix = `--${name}=`;
  const hit = process.argv.find((a) => a.startsWith(prefix));
  return hit ? hit.slice(prefix.length).trim() : null;
}

const dryRun = process.argv.includes("--dry-run");

async function main() {
  if (!isDatabaseConfigured()) {
    throw new Error("DATABASE_URL manquante");
  }
  const slug =
    argValue("tenant")?.trim() ||
    process.env.DEFAULT_TENANT_SLUG?.trim() ||
    "la-providence-nicolas-barre";

  const db = getDb();
  const [etab] = await db
    .select({ id: etablissement.id, slug: etablissement.slug, name: etablissement.name })
    .from(etablissement)
    .where(eq(etablissement.slug, slug))
    .limit(1);
  if (!etab) throw new Error(`Établissement introuvable : ${slug}`);

  console.log(`Tenant ${etab.slug} (${etab.id}) — dryRun=${dryRun}`);

  if (dryRun) {
    const { countCollection } = await import("../app/lib/ent-collection-db.ts");
    const collections = [
      "stages__conventions",
      "stages__offers",
      "stages__sign-tokens",
      "stages__student-tokens",
      "stages__offer-applications",
      "stages__periods",
      "stages__referents",
    ];
    for (const c of collections) {
      const n = await countCollection(etab.id, c);
      console.log(`  collection ${c}: ${n} records`);
    }
    return;
  }

  // Force tenant context for resolveCurrentEtablissementId paths if needed —
  // migrateAllStagesFromCollection takes etablissementId explicitly.
  const { migrateAllStagesFromCollection } = await import("../app/lib/stage-db.ts");
  const result = await migrateAllStagesFromCollection(etab.id);
  console.log("Migration OK:", result);

  const { listConventionsFromDb, listOffersFromDb } = await import("../app/lib/stage-db.ts");
  const conventions = await listConventionsFromDb(etab.id);
  const offers = await listOffersFromDb(etab.id);
  console.log(`Vérif lecture typée : ${conventions.length} conventions, ${offers.length} offres`);

  const borel = conventions.filter((c) =>
    `${c.student?.lastName ?? ""} ${c.student?.firstName ?? ""}`.toLowerCase().includes("borel"),
  );
  for (const c of borel) {
    const dirs = (c.signatures || []).filter((s) => s.role === "direction");
    console.log("BOREL", c.id, c.status, {
      directionSlots: dirs.map((d) => ({
        id: d.id,
        status: d.status,
        signEmail: d.signEmail,
        signedAt: d.signedAt,
        signedBy: d.signedBy,
      })),
    });
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
