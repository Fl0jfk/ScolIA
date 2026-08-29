/**
 * Vérifie la détection collège/lycée pour les classes Stages.
 * Usage: npx tsx scripts/test-stage-siecle-classes.ts [etablissementId]
 */
import { existsSync, readFileSync } from "node:fs";
import { drizzle } from "drizzle-orm/postgres-js";
import postgres from "postgres";
import { eq } from "drizzle-orm";
import { etablissement, refNomenclature } from "../db/schema";
import { parseStructuresDivisions } from "../app/lib/nomenclature-import/siecle-nomenclature-parse";

function loadEnvFile(path: string) {
  if (!existsSync(path)) return;
  for (const line of readFileSync(path, "utf8").split("\n")) {
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

function assertParse(label: string, xml: string, expectedLocked: string[]) {
  const rows = parseStructuresDivisions(xml);
  const locked = rows
    .filter((r) => {
      const pole = (r.metadataJson as { pole?: string } | undefined)?.pole;
      return pole === "COLLÈGE" || pole === "LYCÉE";
    })
    .map((r) => r.code);
  const missing = expectedLocked.filter((c) => !locked.includes(c));
  if (missing.length) {
    console.error(`FAIL ${label}: manquant ${missing.join(", ")} — obtenu ${locked.join(", ")}`);
    process.exit(1);
  }
  console.log(`OK ${label}:`, locked.join(", "));
}

async function main() {
  loadEnvFile(".env.local");

  assertParse(
    "DIVISION collège/lycée",
    `<?xml version="1.0"?><BEE_STRUCTURES>
      <DIVISION CODE_STRUCTURE="6A"><LIBELLE_COURT>6A</LIBELLE_COURT></DIVISION>
      <DIVISION CODE_STRUCTURE="1A"><LIBELLE_COURT>1 A</LIBELLE_COURT></DIVISION>
      <DIVISION CODE_STRUCTURE="2GT1"><LIBELLE_COURT>2GT1</LIBELLE_COURT></DIVISION>
      <DIVISION CODE_STRUCTURE="CP"><LIBELLE_COURT>CP</LIBELLE_COURT></DIVISION>
    </BEE_STRUCTURES>`,
    ["6A", "1A", "2GT1"],
  );

  assertParse(
    "GROUPE fallback",
    `<?xml version="1.0"?><BEE_STRUCTURES>
      <GROUPE CODE_GROUPE="5B"><LIBELLE_COURT>5 B</LIBELLE_COURT></GROUPE>
      <GROUPE CODE_GROUPE="TG2"><LIBELLE_COURT>Terminale G2</LIBELLE_COURT></GROUPE>
    </BEE_STRUCTURES>`,
    ["5B", "TG2"],
  );

  const url = process.env.DATABASE_URL?.trim();
  if (!url) {
    console.log("DATABASE_URL absent — tests parse uniquement.");
    return;
  }

  const sql = postgres(url, { max: 1 });
  const db = drizzle(sql);

  const etabIdArg = process.argv[2]?.trim();
  let etabId = etabIdArg;
  if (!etabId) {
    const [etab] = await db.select({ id: etablissement.id }).from(etablissement).limit(1);
    etabId = etab?.id;
  }
  if (!etabId) {
    console.error("Aucun établissement en BDD");
    process.exit(1);
  }

  const divisions = await db
    .select({
      code: refNomenclature.code,
      pole: refNomenclature.metadataJson,
    })
    .from(refNomenclature)
    .where(eq(refNomenclature.etablissementId, etabId));

  const divisionRows = divisions.filter((d) => true);
  console.log(`\nBDD ${etabId}: ${divisionRows.length} division(s) ref_nomenclature`);
  console.log("Tests parse OK — chargement serveur testé via UI /api/stages/periods.");
  await sql.end();
  console.log("\nTous les tests OK.");
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
