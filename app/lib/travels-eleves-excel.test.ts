import assert from "node:assert/strict";
import test from "node:test";
import * as XLSX from "xlsx";
import {
  buildElevesListExcelRows,
  elevesListExcelFilename,
} from "./travels-eleves-list";
import { buildElevesListXlsxBytes } from "./travels-eleves-excel";

test("buildElevesListExcelRows — tri classe puis nom puis prénom", () => {
  const rows = buildElevesListExcelRows([
    { nom: "Martin", prenom: "Zoé", classe: "5D" },
    { nom: "Martin", prenom: "Anna", classe: "5D" },
    { nom: "Bernard", prenom: "Luc", classe: "5B" },
    { nom: "Dupont", prenom: "Alice", classe: "5B" },
  ]);
  assert.deepEqual(
    rows.map((r) => `${r.Classe}|${r.Nom}|${r.Prénom}`),
    ["5B|Bernard|Luc", "5B|Dupont|Alice", "5D|Martin|Anna", "5D|Martin|Zoé"],
  );
});

test("elevesListExcelFilename — sanitize le libellé", () => {
  const name = elevesListExcelFilename("Sortie Musée / Paris");
  assert.match(name, /^liste-eleves-Sortie-Musee-Paris-\d{4}-\d{2}-\d{2}\.xlsx$/);
});

test("buildElevesListXlsxBytes — feuille lisible avec colonnes attendues", () => {
  const bytes = buildElevesListXlsxBytes([
    { nom: "Dupont", prenom: "Alice", classe: "5B" },
    { nom: "Martin", prenom: "Bob", classe: "5D" },
  ]);
  const wb = XLSX.read(bytes, { type: "array" });
  const sheet = wb.Sheets[wb.SheetNames[0]!];
  assert.ok(sheet);
  const json = XLSX.utils.sheet_to_json<Record<string, string>>(sheet);
  assert.equal(json.length, 2);
  assert.equal(json[0]?.Nom, "Dupont");
  assert.equal(json[0]?.Prénom, "Alice");
  assert.equal(json[0]?.Classe, "5B");
  assert.equal(json[1]?.Classe, "5D");
});
