import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

const ROOT = path.join(import.meta.dirname, "..", "..");

function readRoute(rel: string): string {
  return fs.readFileSync(path.join(ROOT, rel), "utf8");
}

test("bulletins JSON — pas de 400 si date_debut NULL (repli année)", () => {
  const src = readRoute("app/api/notes/bulletins/route.ts");
  assert.ok(src.includes("resolveNotesPeriodeDateDebutIso"));
  assert.equal(src.includes("sans date de début"), false);
});

test("bulletins PDF/ZIP — pas de 400 si date_debut NULL", () => {
  const src = readRoute("app/api/notes/bulletins/pdf/route.ts");
  assert.ok(src.includes("resolveNotesPeriodeDateDebutIso"));
  assert.equal(src.includes("sans date de début"), false);
});

test("saisie notes — repli période via resolvePeriodeDateDebutForNotesLists", () => {
  const src = readRoute("app/api/notes/saisie/route.ts");
  assert.ok(src.includes("resolvePeriodeDateDebutForNotesLists"));
});

test("compétences — filtre période avec repli", () => {
  const src = readRoute("app/api/notes/competences/route.ts");
  assert.ok(src.includes("resolvePeriodeDateDebutForNotesLists"));
  assert.ok(src.includes("periodeDateDebut"));
  assert.ok(src.includes("isUuidV4Like"));
});

test("routes notes — periodeId invalide → 400", () => {
  for (const rel of [
    "app/api/notes/saisie/route.ts",
    "app/api/notes/competences/route.ts",
    "app/api/notes/bulletins/pdf/route.ts",
  ]) {
    const src = readRoute(rel);
    assert.ok(src.includes("isUuidV4Like") || src.includes("InvalidPeriodeIdError"), rel);
  }
});
