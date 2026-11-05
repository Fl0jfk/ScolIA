import assert from "node:assert/strict";
import test from "node:test";
import * as XLSX from "xlsx";
import {
  matchBirthdateRows,
  nameKey,
  parseBirthDateCell,
  parseBirthdateSheetBuffer,
  type BirthdateStaff,
} from "@/app/lib/personnel-birthdate-import";

const staff: BirthdateStaff[] = [
  { id: "1", firstName: "Jean-Pierre", lastName: "Dupont", emails: ["jp@x.fr"], birthDate: null },
  { id: "2", firstName: "Éloïse", lastName: "Martin", emails: ["eloise@x.fr"], birthDate: "1980-01-01" },
  { id: "3", firstName: "Anne", lastName: "Leroy", emails: [], birthDate: "1975-05-05" },
  { id: "4", firstName: "Paul", lastName: "Durand", emails: [], birthDate: null },
  { id: "5", firstName: "Paul", lastName: "Durand", emails: [], birthDate: null },
];

test("dates", () => {
  assert.equal(parseBirthDateCell("12/03/1985"), "1985-03-12");
  assert.equal(parseBirthDateCell("12-03-1985"), "1985-03-12");
  assert.equal(parseBirthDateCell("1985-03-12"), "1985-03-12");
  assert.equal(parseBirthDateCell(45000), "2023-03-15");
  assert.equal(parseBirthDateCell("n/a"), null);
});

test("nameKey tolère ordre, accents, tirets", () => {
  assert.equal(nameKey("Éloïse", "MARTIN"), nameKey("martin eloise"));
  assert.equal(nameKey("Jean-Pierre", "Dupont"), nameKey("DUPONT Jean Pierre"));
});

test("statuts", () => {
  const row = (line: number, firstName: string, lastName: string, rawDate: string, fullName = "", email = "") => ({
    line,
    firstName,
    lastName,
    fullName,
    email,
    rawDate,
  });
  const rows = matchBirthdateRows(
    [
      row(2, "Jean Pierre", "DUPONT", "01/02/1970"),
      row(3, "", "", "1980-01-01", "Martin Eloise"),
      row(4, "Anne", "Leroy", "06/06/1975"),
      row(5, "Paul", "Durand", "06/06/1975"),
      row(6, "Inconnu", "Zed", "06/06/1975"),
      row(7, "Anne", "Leroy", "pas une date"),
      row(8, "X", "Y", "01/02/1971", "", "jp@x.fr"),
    ],
    staff,
  );
  assert.deepEqual(
    rows.map((r) => r.status),
    ["ok", "inchange", "remplace", "ambigu", "introuvable", "date_invalide", "ambigu"],
  );
  assert.equal(rows[2]!.oldDate, "1975-05-05");
});

test("parse xlsx avec en-têtes souples", () => {
  const ws = XLSX.utils.aoa_to_sheet([
    ["Nom", "Prénom", "E-mail", "Né le"],
    ["Dupont", "Jean-Pierre", "", "02/01/1970"],
  ]);
  const wb = XLSX.utils.book_new();
  XLSX.utils.book_append_sheet(wb, ws, "S");
  const buf = XLSX.write(wb, { type: "array", bookType: "xlsx" }) as ArrayBuffer;
  const p = parseBirthdateSheetBuffer(buf);
  assert.ok(p.ok);
  if (p.ok) {
    assert.equal(p.rows.length, 1);
    assert.equal(p.rows[0]!.lastName, "Dupont");
    assert.equal(p.rows[0]!.firstName, "Jean-Pierre");
    assert.equal(p.rows[0]!.fullName, "");
  }
});

test("trois colonnes Nom / Prénom / Date de naissance : la ligne est reconnue", () => {
  const ws = XLSX.utils.aoa_to_sheet([
    ["NOM", "Prénom", "Date de naissance"],
    ["Dupont", "Jean-Pierre", "02/01/1970"],
  ]);
  const wb = XLSX.utils.book_new();
  XLSX.utils.book_append_sheet(wb, ws, "S");
  const buf = XLSX.write(wb, { type: "array", bookType: "xlsx" }) as ArrayBuffer;
  const p = parseBirthdateSheetBuffer(buf);
  assert.ok(p.ok);
  if (!p.ok) return;
  assert.equal(p.rows[0]!.line, 2);
  assert.equal(p.rows[0]!.firstName, "Jean-Pierre");
  const [r] = matchBirthdateRows(p.rows, staff);
  assert.equal(r!.status, "ok");
  assert.equal(r!.newDate, "1970-01-02");
});

test("colonne unique « Nom Prénom » reste un nom complet", () => {
  const ws = XLSX.utils.aoa_to_sheet([
    ["Nom Prénom", "Date de naissance"],
    ["DUPONT Jean-Pierre", "02/01/1970"],
  ]);
  const wb = XLSX.utils.book_new();
  XLSX.utils.book_append_sheet(wb, ws, "S");
  const buf = XLSX.write(wb, { type: "array", bookType: "xlsx" }) as ArrayBuffer;
  const p = parseBirthdateSheetBuffer(buf);
  assert.ok(p.ok);
  if (!p.ok) return;
  assert.equal(p.rows[0]!.fullName, "DUPONT Jean-Pierre");
  assert.equal(matchBirthdateRows(p.rows, staff)[0]!.status, "ok");
});
