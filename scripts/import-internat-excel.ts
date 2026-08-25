/**
 * Importe un Excel Charlemagne / Pronote → référentiel élèves + roster internat.
 * Ne lit jamais IBAN / BIC / RUM (colonnes absentes du parseur).
 *
 * Usage :
 *   npx tsx --require ./scripts/stub-server-only.cjs scripts/import-internat-excel.ts \
 *     --file="C:/path/Classeur.xlsx" \
 *     --tenant=la-providence-nicolas-barre
 */
import { existsSync, readFileSync } from "node:fs";
import { eq, sql } from "drizzle-orm";
import {
  putCollectionSingleton,
  getCollectionRecord,
  upsertCollectionRecord,
} from "../app/lib/ent-collection-db";
import { isRegimeInterne } from "../app/lib/eleve-regime";
import { mergeElevesLists, parseElevesExcelBuffer } from "../app/lib/eleves-import";
import {
  applyInternatRoster,
  elevesAsInternatRosterEntries,
  elevesToInternatRosterEntries,
  INTERNAT_ROSTER_KEY,
  type InternatRosterEntry,
  type InternatRosterFile,
} from "../app/lib/internat-import";
import { INTERNAT_S3, type InternatStudent } from "../app/lib/internat-types";
import { jsonPathToCollection } from "../app/lib/ent-json-postgres";
import {
  listElevesFromDb,
  replaceElevesInDb,
} from "../app/lib/ent-core-db";
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

function inferEtablissement(classe: string | undefined): string {
  const c = String(classe ?? "")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .trim();
  if (!c) return "Lycée";
  if (/^(6|5|4|3)/.test(c) || c.includes("college") || /[3456]e/.test(c)) {
    return "Collège";
  }
  return "Lycée";
}

async function putDoc(etablissementId: string, relativePath: string, data: unknown) {
  const { collection, recordId, singleton } = jsonPathToCollection(relativePath);
  if (singleton) {
    await putCollectionSingleton(etablissementId, collection, data);
    return;
  }
  if (!data || typeof data !== "object" || Array.isArray(data)) {
    await upsertCollectionRecord(etablissementId, collection, recordId, {
      __root: data as unknown,
    });
    return;
  }
  const obj = { ...(data as Record<string, unknown>) };
  if (obj.id == null) obj.id = recordId;
  await upsertCollectionRecord(etablissementId, collection, recordId, obj);
}

async function loadStudents(etablissementId: string): Promise<InternatStudent[]> {
  const row = await getCollectionRecord<Record<string, unknown>>(
    etablissementId,
    "internat",
    "students",
  );
  if (!row) return [];
  if ("__root" in row && Array.isArray(row.__root)) {
    return row.__root as InternatStudent[];
  }
  return [];
}

async function main() {
  if (!isDatabaseConfigured()) {
    throw new Error("DATABASE_URL manquant");
  }
  process.env.ENT_CORE_DB = process.env.ENT_CORE_DB || "1";
  process.env.ENT_IMPORT_SCRIPT = "1";
  process.env.NODE_TLS_REJECT_UNAUTHORIZED =
    process.env.NODE_TLS_REJECT_UNAUTHORIZED || "0";

  const filePath = argValue("file");
  const slug =
    argValue("tenant") ||
    process.env.DEFAULT_TENANT_SLUG?.trim() ||
    "la-providence-nicolas-barre";

  if (!filePath) {
    throw new Error(
      'Usage: --file="C:/path/export.xlsx" [--tenant=la-providence-nicolas-barre]',
    );
  }
  if (!existsSync(filePath)) {
    throw new Error(`Fichier introuvable : ${filePath}`);
  }

  const buf = readFileSync(filePath);
  const parsed = parseElevesExcelBuffer(
    buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength),
    "auto",
  );
  if (!parsed.ok) {
    throw new Error(parsed.error);
  }

  const withRegime = parsed.eleves.filter((e) => e.regime?.trim());
  const internes = parsed.eleves.filter((e) => isRegimeInterne(e.regime));
  const entries: InternatRosterEntry[] =
    withRegime.length > 0
      ? elevesToInternatRosterEntries(parsed.eleves)
      : elevesAsInternatRosterEntries(parsed.eleves);

  for (const entry of entries) {
    if (!entry.etablissement) {
      entry.etablissement = inferEtablissement(entry.classe);
    }
  }

  console.log(
    JSON.stringify(
      {
        file: filePath,
        tenant: slug,
        parsed: parsed.eleves.length,
        withRegime: withRegime.length,
        internesDetected: internes.length,
        rosterEntries: entries.length,
        source: parsed.detectedSource,
        headerRow: parsed.headerRow,
      },
      null,
      2,
    ),
  );

  if (!entries.length) {
    throw new Error("Aucun interne détecté — vérifiez la colonne Régime (INT / Interne).");
  }

  const db = getDb();
  const [etab] = await db
    .select()
    .from(etablissement)
    .where(eq(etablissement.slug, slug))
    .limit(1);
  if (!etab) {
    throw new Error(`Établissement introuvable pour le slug « ${slug} ».`);
  }

  // Migration légère (réseau lycée : parfois db:migrate timeout).
  await db.execute(sql.raw(`ALTER TABLE "eleve" ADD COLUMN IF NOT EXISTS "regime" text`));
  await db.execute(sql.raw(`ALTER TABLE "eleve" ADD COLUMN IF NOT EXISTS "sexe" text`));
  await db.execute(
    sql.raw(
      `CREATE INDEX IF NOT EXISTS "eleve_etablissement_regime_idx" ON "eleve" ("etablissement_id", "regime")`,
    ),
  );

  const existing = await listElevesFromDb(etab.id);
  const toMerge =
    withRegime.length > 0
      ? parsed.eleves
      : parsed.eleves.map((e) => ({ ...e, regime: e.regime || "Interne" }));
  const merged = mergeElevesLists(existing, toMerge).eleves;

  await replaceElevesInDb(etab.id, merged);
  await putDoc(etab.id, "eleves.json", merged);

  const now = new Date().toISOString();
  const appliedBy = "import-internat-excel";
  const students = await loadStudents(etab.id);
  const result = await applyInternatRoster({
    entries,
    students,
    appliedBy,
  });

  const roster: InternatRosterFile = {
    meta: {
      updatedAt: now,
      updatedBy: appliedBy,
      count: entries.length,
      lastAppliedAt: now,
      lastAppliedBy: appliedBy,
      lastApplySummary: {
        added: result.added,
        updated: result.updated,
        skipped: result.skipped,
        sorties: result.sorties,
        reactivated: result.reactivated,
      },
    },
    entries,
  };

  await putDoc(etab.id, INTERNAT_ROSTER_KEY, roster);
  await putDoc(etab.id, INTERNAT_S3.students, result.students);

  console.log(
    JSON.stringify(
      {
        elevesInDb: merged.length,
        rosterCount: entries.length,
        added: result.added,
        updated: result.updated,
        skipped: result.skipped,
        sorties: result.sorties,
        reactivated: result.reactivated,
        studentsActifs: result.students.filter((s) => s.actif).length,
      },
      null,
      2,
    ),
  );
}

main()
  .catch((err) => {
    console.error(err instanceof Error ? err.message : err);
    process.exitCode = 1;
  })
  .finally(async () => {
    await closeDb().catch(() => undefined);
  });
