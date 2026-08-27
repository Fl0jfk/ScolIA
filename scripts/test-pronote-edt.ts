/**
 * Test rapide parseur EDT Pronote spatial.
 * Usage:
 *   npx tsx --require ./scripts/stub-server-only.cjs scripts/test-pronote-edt.ts [chemin.pdf]
 */
import fs from "fs";
import path from "path";
import { extractPdfTextItems } from "@/app/lib/rh/planning-pdf-text";
import {
  looksLikePronoteTeacherPdf,
  parsePronoteTeacherGrid,
} from "@/app/lib/rh/planning-pronote-parse";
import { normalizeTeacherPlanning } from "@/app/lib/rh/planning-types";

const pdfPath =
  process.argv[2] ||
  path.join(
    process.env.USERPROFILE || "",
    ".cursor/projects/c-Users-f-hacqueville-Desktop-Code-docsLaPro/attachments/53b358f9-cf3b-42cc-bea7-b15b157be3d6/Planning_Val_rie.pdf",
  );

async function main() {
  if (!fs.existsSync(pdfPath)) {
    console.error("PDF introuvable:", pdfPath);
    process.exit(1);
  }
  const bytes = fs.readFileSync(pdfPath);
  const items = await extractPdfTextItems(bytes);
  console.log("items", items.length, "looksPronote", looksLikePronoteTeacherPdf(items));
  const parsed = parsePronoteTeacherGrid(items);
  if (!parsed) {
    console.error("parse null");
    process.exit(2);
  }
  console.log("personHint", parsed.personHint);
  console.log("slotCount", parsed.slotCount);
  console.log("weekA", parsed.weekA.length, "weekB", parsed.weekB.length);
  console.log("warnings", parsed.warnings);
  if (parsed.weekB.length < parsed.weekA.length - 5) {
    console.error("FAIL: weekB trop petit vs weekA — les créneaux sans marqueur doivent être dans les deux semaines");
    process.exit(3);
  }
  const planning = normalizeTeacherPlanning(
    { kind: "teacher", personnelId: "test", weekA: parsed.weekA, weekB: parsed.weekB },
    "test",
  );
  const idsA = new Set(planning.weekA.map((s) => s.id));
  const shared = planning.weekB.filter((s) => idsA.has(s.id));
  if (shared.length) {
    console.error("FAIL: ids partagés entre weekA et weekB", shared.length);
    process.exit(4);
  }
  console.log("normalize ok — ids A/B disjoints, weekA", planning.weekA.length, "weekB", planning.weekB.length);
  const byDay = (slots: typeof planning.weekA) => {
    const m = new Map<number, string[]>();
    for (const s of slots) {
      const list = m.get(s.day) || [];
      list.push(
        `${s.start}-${s.end} ${s.subject} [${s.classes.join(",")}] ${s.room || ""}`.trim(),
      );
      m.set(s.day, list);
    }
    return Object.fromEntries([...m.entries()].sort((a, b) => a[0] - b[0]));
  };
  console.log("WEEK A", JSON.stringify(byDay(planning.weekA), null, 2));
  console.log("WEEK B only extras vs A count", planning.weekB.length);
  const aKeys = new Set(
    planning.weekA.map((s) => `${s.day}|${s.start}|${s.end}|${s.subject}|${s.classes.join(",")}`),
  );
  const bOnly = planning.weekB.filter(
    (s) => !aKeys.has(`${s.day}|${s.start}|${s.end}|${s.subject}|${s.classes.join(",")}`),
  );
  console.log("B-only slots", bOnly);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
