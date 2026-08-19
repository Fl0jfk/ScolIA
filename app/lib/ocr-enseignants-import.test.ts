import assert from "node:assert/strict";
import test from "node:test";
import {
  inferSecteursFromClasses,
  mergeEnseignantsLists,
  parseEnseignantsExcelBuffer,
} from "./ocr-enseignants-import";
import * as XLSX from "xlsx";

function sheetToBuffer(rows: unknown[][]): ArrayBuffer {
  const ws = XLSX.utils.aoa_to_sheet(rows);
  const wb = XLSX.utils.book_new();
  XLSX.utils.book_append_sheet(wb, ws, "Feuil1");
  return XLSX.write(wb, { type: "array", bookType: "xlsx" }) as ArrayBuffer;
}

test("infère collège et lycée depuis les classes Charlemagne", () => {
  assert.deepEqual(inferSecteursFromClasses("3°E, TB, 2A"), ["college", "lycee"]);
  assert.deepEqual(inferSecteursFromClasses("C.E.2 MME MAHIEUX"), ["ecole"]);
  assert.deepEqual(inferSecteursFromClasses("1D"), ["lycee"]);
});

test("parse un export Excel type Charlemagne avec deux emails", () => {
  const buffer = sheetToBuffer([
    ["Nom", "Prénom", "Liste des classes", "Email personnel", "Email professionnel"],
    ["HEBERT", "Pascal", "1C, TB, 2E", "perso@example.fr", "p.hebert@ecole.fr"],
    ["MAHIEUX", "Fabienne", "C.E.2 MME MAHIEUX", "mahieux@example.fr", ""],
  ]);
  const parsed = parseEnseignantsExcelBuffer(buffer);
  assert.equal(parsed.ok, true);
  if (!parsed.ok) return;
  const hebert = parsed.enseignants.find((e) => e.nom === "HEBERT");
  assert.equal(hebert?.email, "perso@example.fr");
  assert.equal(hebert?.emailPro, "p.hebert@ecole.fr");
});

test("fusionne les listes enseignants par nom+prénom+cycle", () => {
  const merged = mergeEnseignantsLists(
    [{ id: "1", nom: "A", prenom: "B", folderName: "A B", secteur: "college" }],
    [{ id: "", nom: "A", prenom: "B", folderName: "", secteur: "college", email: "a@x.fr" }],
  );
  assert.equal(merged.stats.updated, 1);
  assert.equal(merged.stats.added, 0);
  assert.equal(merged.enseignants[0]?.email, "a@x.fr");
});

test("parse un export Excel type Charlemagne", () => {
  const buffer = sheetToBuffer([
    ["Nom", "Prénom", "Liste des classes", "Email personnel"],
    ["HEBERT", "Pascal", "1C, TB, 2E", ""],
    ["MAHIEUX", "Fabienne", "C.E.2 MME MAHIEUX", "mahieux@example.fr"],
  ]);
  const parsed = parseEnseignantsExcelBuffer(buffer);
  assert.equal(parsed.ok, true);
  if (!parsed.ok) return;
  assert.equal(parsed.enseignants.length, 2);
  assert.equal(parsed.enseignants.some((e) => e.nom === "HEBERT" && e.secteur === "lycee"), true);
  assert.equal(parsed.enseignants.some((e) => e.nom === "MAHIEUX" && e.secteur === "ecole"), true);
});
