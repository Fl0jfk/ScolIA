/**
 * Test parsers Siècle sur fichiers locaux (sans import BDD).
 * Usage: npx tsx scripts/test-siecle-nomenclature-import.ts [dir]
 */
import fs from "node:fs";
import path from "node:path";
import { decodeSiecleBuffer } from "@/app/lib/nomenclature-import/siecle-xml-parse-utils";
import { parseNomenclatureXml, parseStructuresDivisions } from "@/app/lib/nomenclature-import/siecle-nomenclature-parse";
import { parseGeographiqueXml } from "@/app/lib/nomenclature-import/siecle-geographique-parse";

const defaultDir = "/home/ubuntu/.cursor/projects/workspace/uploads";

async function main() {
  const dir = process.argv[2] || defaultDir;
  const files = fs.readdirSync(dir).filter((f) => f.toLowerCase().endsWith(".xml"));
  if (!files.length) {
    console.error("Aucun XML dans", dir);
    process.exit(1);
  }

  console.log("=== Parse dry-run ===");
  for (const name of files) {
    const buf = fs.readFileSync(path.join(dir, name));
    const xml = decodeSiecleBuffer(buf);
    if (name.toLowerCase().includes("nomenclature")) {
      const rows = parseNomenclatureXml(xml);
      const byType = rows.reduce<Record<string, number>>((acc, r) => {
        acc[r.type] = (acc[r.type] || 0) + 1;
        return acc;
      }, {});
      console.log(name, "→", rows.length, "entrées", byType);
    } else if (name.toLowerCase().includes("geograph")) {
      const rows = parseGeographiqueXml(xml);
      const byType = rows.reduce<Record<string, number>>((acc, r) => {
        acc[r.type] = (acc[r.type] || 0) + 1;
        return acc;
      }, {});
      console.log(name, "→", rows.length, "entrées", byType);
    } else if (name.toLowerCase().includes("structure")) {
      const rows = parseStructuresDivisions(xml);
      console.log(name, "→", rows.length, "divisions", rows.map((r) => r.code).join(", "));
    } else {
      console.log(name, "→", "détection import batch");
    }
  }

  console.log("\n=== Assertions ===");
  const nomFile = files.find((f) => f.toLowerCase().includes("nomenclature"));
  if (nomFile) {
    const xml = decodeSiecleBuffer(fs.readFileSync(path.join(dir, nomFile)));
    const rows = parseNomenclatureXml(xml);
    const mef = rows.filter((r) => r.type === "mef").length;
    const matiere = rows.filter((r) => r.type === "matiere").length;
    if (mef < 1 || matiere < 1) {
      console.error("FAIL: MEF ou matières vides après parse");
      process.exit(1);
    }
    console.log("OK nomenclature:", mef, "MEF,", matiere, "matières");
  }

  const structFile = files.find((f) => f.toLowerCase().includes("structure"));
  if (structFile) {
    const xml = decodeSiecleBuffer(fs.readFileSync(path.join(dir, structFile)));
    const rows = parseStructuresDivisions(xml);
    if (rows.length < 1) {
      console.error("FAIL: aucune division parsée");
      process.exit(1);
    }
    console.log("OK structures:", rows.length, "divisions");
  }

  const geoFile = files.find((f) => f.toLowerCase().includes("geograph"));
  if (geoFile) {
    const xml = decodeSiecleBuffer(fs.readFileSync(path.join(dir, geoFile)));
    const rows = parseGeographiqueXml(xml);
    const communes = rows.filter((r) => r.type === "commune").length;
    if (communes < 100) {
      console.error("FAIL: communes insuffisantes", communes);
      process.exit(1);
    }
    console.log("OK geographique:", communes, "communes");
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
