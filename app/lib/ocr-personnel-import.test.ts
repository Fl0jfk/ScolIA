import assert from "node:assert/strict";
import test from "node:test";
import * as XLSX from "xlsx";
import { parsePersonnelExcelBuffer } from "./ocr-personnel-import";

function sheetToBuffer(rows: unknown[][]): ArrayBuffer {
  const ws = XLSX.utils.aoa_to_sheet(rows);
  const wb = XLSX.utils.book_new();
  XLSX.utils.book_append_sheet(wb, ws, "Feuil1");
  return XLSX.write(wb, { type: "array", bookType: "xlsx" }) as ArrayBuffer;
}

test("parse personnel avec Nom, Prénom, emails et Fonction", () => {
  const buffer = sheetToBuffer([
    ["Nom", "Prénom", "Email personnel", "Email professionnel", "Fonction"],
    ["DUPONT", "Marie", "marie@gmail.com", "m.dupont@ecole.fr", "Secrétaire"],
    ["MARTIN", "Paul", "", "p.martin@ecole.fr", "Comptable"],
  ]);
  const parsed = parsePersonnelExcelBuffer(buffer);
  assert.equal(parsed.ok, true);
  if (!parsed.ok) return;
  assert.equal(parsed.rows.length, 2);
  assert.equal(parsed.rows[0]?.jobTitle, "Secrétaire");
  assert.equal(parsed.rows[0]?.category, "administratif");
  assert.equal(parsed.rows[1]?.category, "comptabilite");
  assert.equal(parsed.rows[1]?.emailPro, "p.martin@ecole.fr");
});
