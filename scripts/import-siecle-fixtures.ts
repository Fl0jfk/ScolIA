/**
 * Import XML Siècle locaux en BDD (script CLI, sans server-only).
 * Usage: npx tsx scripts/import-siecle-fixtures.ts [dir]
 */
import fs from "node:fs";
import path from "node:path";
import { existsSync, readFileSync } from "node:fs";
import { and, eq } from "drizzle-orm";
import { drizzle } from "drizzle-orm/postgres-js";
import postgres from "postgres";
import { etablissement, refNomenclature } from "../db/schema";
import { decodeSiecleBuffer } from "../app/lib/nomenclature-import/siecle-xml-parse-utils";
import { parseGeographiqueXml } from "../app/lib/nomenclature-import/siecle-geographique-parse";
import {
  parseNomenclatureXml,
  parseStructuresDivisions,
} from "../app/lib/nomenclature-import/siecle-nomenclature-parse";
import type { NomenclatureUpsertRow } from "../app/lib/nomenclature-import/siecle-xml-types";

function loadEnvFile(envPath: string) {
  if (!existsSync(envPath)) return;
  for (const line of readFileSync(envPath, "utf8").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const eqIdx = trimmed.indexOf("=");
    if (eqIdx <= 0) continue;
    const key = trimmed.slice(0, eqIdx).trim();
    let val = trimmed.slice(eqIdx + 1).trim();
    if ((val.startsWith('"') && val.endsWith('"')) || (val.startsWith("'") && val.endsWith("'"))) {
      val = val.slice(1, -1);
    }
    if (!process.env[key]) process.env[key] = val;
  }
}

async function upsertRows(
  db: ReturnType<typeof drizzle>,
  etablissementId: string,
  rows: NomenclatureUpsertRow[],
): Promise<{ inserts: number; updates: number }> {
  let inserts = 0;
  let updates = 0;
  for (const r of rows) {
    const existing = await db
      .select({ id: refNomenclature.id })
      .from(refNomenclature)
      .where(
        and(
          eq(refNomenclature.etablissementId, etablissementId),
          eq(refNomenclature.type, r.type),
          eq(refNomenclature.code, r.code),
        ),
      )
      .limit(1);

    if (existing[0]) {
      await db
        .update(refNomenclature)
        .set({
          libelleCourt: r.libelleCourt || null,
          libelleLong: r.libelleLong || null,
          metadataJson: r.metadataJson || null,
          validFrom: r.validFrom || null,
          validTo: r.validTo || null,
          updatedAt: new Date(),
        })
        .where(eq(refNomenclature.id, existing[0].id));
      updates += 1;
    } else {
      await db.insert(refNomenclature).values({
        etablissementId,
        type: r.type,
        code: r.code,
        libelleCourt: r.libelleCourt || null,
        libelleLong: r.libelleLong || null,
        metadataJson: r.metadataJson || null,
        validFrom: r.validFrom || null,
        validTo: r.validTo || null,
      });
      inserts += 1;
    }
  }
  return { inserts, updates };
}

async function main() {
  loadEnvFile(path.join(process.cwd(), ".env.local"));
  const url = process.env.DATABASE_URL;
  if (!url) throw new Error("DATABASE_URL manquant");

  const dir = process.argv[2] || "/home/ubuntu/.cursor/projects/workspace/uploads";
  const sql = postgres(url, { max: 1 });
  const db = drizzle(sql);

  const [etab] = await db.select({ id: etablissement.id }).from(etablissement).limit(1);
  if (!etab) throw new Error("Aucun établissement — npm run seed:dev");

  const files = fs.readdirSync(dir).filter((f) => f.toLowerCase().endsWith(".xml"));
  const ordered = [...files].sort((a, b) => {
    const rank = (n: string) => {
      const x = n.toLowerCase();
      if (x.includes("commun")) return 0;
      if (x.includes("nomenclature")) return 1;
      if (x.includes("geograph")) return 2;
      if (x.includes("structure")) return 3;
      return 9;
    };
    return rank(a) - rank(b);
  });

  for (const name of ordered) {
    const buf = fs.readFileSync(path.join(dir, name));
    const xml = decodeSiecleBuffer(buf);
    const head = xml.slice(0, 4000).toUpperCase();

    if (head.includes("BEE_COMMUN")) {
      console.log(name, "— communs (import via UI/API)");
      continue;
    }

    let rows: NomenclatureUpsertRow[] = [];
    if (head.includes("BEE_NOMENCLATURES")) rows = parseNomenclatureXml(xml);
    else if (head.includes("BEE_GEOGRAPHIQUE")) rows = parseGeographiqueXml(xml);
    else if (head.includes("BEE_STRUCTURES")) rows = parseStructuresDivisions(xml);
    else {
      console.log(name, "— ignoré");
      continue;
    }

    const { inserts, updates } = await upsertRows(db, etab.id, rows);
    console.log(`${name} : ${rows.length} entrées (+${inserts} / ~${updates})`);
  }

  const summary = await db
    .select({ type: refNomenclature.type, n: refNomenclature.id })
    .from(refNomenclature)
    .where(eq(refNomenclature.etablissementId, etab.id));

  const byType: Record<string, number> = {};
  for (const row of summary) {
    byType[row.type] = (byType[row.type] || 0) + 1;
  }
  console.log("\nBDD ref_nomenclature:", byType);

  await sql.end();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
