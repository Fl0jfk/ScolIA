/**
 * Import ElevesSansAdresses.xml en prod → régimes BCN + resync internat (scopé au fichier).
 *
 * NE PAS utiliser replaceElevesInDb (wipe collège). Upsert unitaire uniquement.
 *
 * Usage :
 *   npx tsx --require ./scripts/stub-server-only.cjs scripts/import-siecle-eleves-internat.ts \
 *     --file="C:/Users/.../ElevesSansAdresses.xml" \
 *     --tenant=la-providence-nicolas-barre
 */
import { existsSync, readFileSync } from "node:fs";
import { eq } from "drizzle-orm";
import {
  getCollectionRecord,
  putCollectionSingleton,
  upsertCollectionRecord,
} from "../app/lib/ent-collection-db";
import { eleveMatchKey, mergeElevesLists } from "../app/lib/eleves-import";
import {
  applyInternatRoster,
  elevesToInternatRosterEntries,
  INTERNAT_ROSTER_KEY,
  type InternatRosterFile,
} from "../app/lib/internat-import";
import { INTERNAT_S3, type InternatStudent } from "../app/lib/internat-types";
import { jsonPathToCollection } from "../app/lib/ent-json-postgres";
import { listElevesFromDb, upsertElevesInDb } from "../app/lib/ent-core-db";
import { decodeSiecleBuffer } from "../app/lib/nomenclature-import/siecle-xml-parse-utils";
import { parseSiecleElevesXmlServer } from "../app/lib/siecle-eleves-parse";
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
process.env.ENT_IMPORT_SCRIPT = "1";
process.env.ENT_CORE_DB = process.env.ENT_CORE_DB || "1";
process.env.NODE_TLS_REJECT_UNAUTHORIZED =
  process.env.NODE_TLS_REJECT_UNAUTHORIZED || "0";

function argValue(name: string): string | null {
  const prefix = `--${name}=`;
  const hit = process.argv.find((a) => a.startsWith(prefix));
  return hit ? hit.slice(prefix.length).trim() : null;
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

function scopeKeyFromEleve(e: {
  ine?: string;
  nom: string;
  prenom: string;
}): string {
  const ine = e.ine?.trim().toUpperCase();
  if (ine) return `ine:${ine}`;
  return `name:${e.nom.trim().toUpperCase()}|${e.prenom.trim().toUpperCase()}`;
}

async function main() {
  if (!isDatabaseConfigured()) {
    throw new Error("DATABASE_URL manquant (.env)");
  }

  const filePath =
    argValue("file") || "C:/Users/f.hacqueville/Desktop/ElevesSansAdresses.xml";
  const slug =
    argValue("tenant") ||
    process.env.DEFAULT_TENANT_SLUG?.trim() ||
    "la-providence-nicolas-barre";

  if (!existsSync(filePath)) {
    throw new Error(`Fichier introuvable : ${filePath}`);
  }

  const buf = readFileSync(filePath);
  const xml = decodeSiecleBuffer(buf);
  const parsed = parseSiecleElevesXmlServer(xml);

  console.log(
    JSON.stringify(
      {
        file: filePath,
        tenant: slug,
        totalInFile: parsed.totalInFile,
        skippedSortis: parsed.skippedSortis,
        scolarises: parsed.total,
        internes: parsed.internesCount,
        withRegime: parsed.withRegimeCount,
        mapSize: Object.keys(parsed.siecleEleveIdMap).length,
      },
      null,
      2,
    ),
  );

  if (!parsed.eleves.length && !parsed.sortis.length) {
    throw new Error("Aucun élève lu dans le XML.");
  }
  if (parsed.internesCount === 0) {
    throw new Error("0 interne (CODE_REGIME 3) — abort pour ne pas vider l'internat.");
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

  const existing = await listElevesFromDb(etab.id);
  const incoming = [...parsed.eleves, ...parsed.sortis];
  const { eleves: mergedFull, stats } = mergeElevesLists(existing, incoming, {
    replaceRegime: true,
  });
  const touched = new Set(incoming.map((e) => eleveMatchKey(e)));
  const toUpsert = mergedFull.filter((e) => touched.has(eleveMatchKey(e)));

  const upsertStats = await upsertElevesInDb(etab.id, toUpsert);

  // Map ELEVE_ID → INE (feuille __root)
  if (Object.keys(parsed.siecleEleveIdMap).length) {
    await putDoc(etab.id, "siecle/eleve-id-map.json", {
      __root: parsed.siecleEleveIdMap,
    });
  }

  const entries = elevesToInternatRosterEntries(parsed.eleves);
  for (const entry of entries) {
    if (!entry.etablissement) entry.etablissement = "Lycée";
  }

  const sortieScopeKeys = new Set(incoming.map((e) => scopeKeyFromEleve(e)));
  const students = await loadStudents(etab.id);
  const beforeActifs = students.filter((s) => s.actif).length;
  const appliedBy = "import-siecle-eleves-internat";
  const result = await applyInternatRoster({
    entries,
    students,
    appliedBy,
    sortieScopeKeys,
  });

  const now = new Date().toISOString();
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

  const afterActifs = result.students.filter((s) => s.actif).length;

  console.log(
    JSON.stringify(
      {
        etablissementId: etab.id,
        elevesExisting: existing.length,
        elevesUpserted: toUpsert.length,
        mergeStats: stats,
        upsertStats,
        internatBeforeActifs: beforeActifs,
        internatAfterActifs: afterActifs,
        rosterInternesFichier: entries.length,
        added: result.added,
        updated: result.updated,
        skipped: result.skipped,
        sorties: result.sorties,
        reactivated: result.reactivated,
        mapSaved: Object.keys(parsed.siecleEleveIdMap).length,
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
