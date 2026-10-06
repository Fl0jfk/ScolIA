import assert from "node:assert/strict";
import test from "node:test";
import {
  defaultSchoolYearStartIsoFromLabel,
  formatSqlDateToIso,
  isUuidV4Like,
  pickCalendrierAnneeDebutIso,
} from "./notes-periode-debut-logic";
import { isEleveVisiblePourPeriodeNotes } from "./eleve-actif-shared";
import { InvalidPeriodeIdError, resolveNotesPeriodeDateDebutIso } from "./notes-periode-debut";

test("defaultSchoolYearStartIsoFromLabel", () => {
  assert.equal(defaultSchoolYearStartIsoFromLabel("2025-2026"), "2025-09-01");
});

test("formatSqlDateToIso", () => {
  assert.equal(formatSqlDateToIso("2026-01-15"), "2026-01-15");
  assert.equal(formatSqlDateToIso(null), null);
});

test("élève sorti après début année reste visible pour période sans date_debut (repli 1er sept)", () => {
  const debutAnnee = "2025-09-01";
  assert.equal(isEleveVisiblePourPeriodeNotes("2026-06-30", debutAnnee), true);
  assert.equal(isEleveVisiblePourPeriodeNotes("2025-08-31", debutAnnee), false);
});

test("bulletins — pas d’erreur si date_debut période absente : repli calculé", () => {
  const periodeDateDebut = null;
  const repli = defaultSchoolYearStartIsoFromLabel("2025-2026");
  const pivot = periodeDateDebut ?? repli;
  assert.equal(pivot, "2025-09-01");
  assert.equal(isEleveVisiblePourPeriodeNotes("2026-02-01", pivot), true);
});

test("pickCalendrierAnneeDebutIso — ignore les vacances", () => {
  const iso = pickCalendrierAnneeDebutIso([
    { dateDebut: "2025-07-01", type: "vacances" },
    { dateDebut: "2025-09-02", type: "rentree" },
    { dateDebut: "2025-10-20", type: "vacances" },
  ]);
  assert.equal(iso, "2025-09-02");
});

test("isUuidV4Like — rejette les identifiants invalides", () => {
  assert.equal(isUuidV4Like("not-a-uuid"), false);
  assert.equal(isUuidV4Like("550e8400-e29b-41d4-a716-446655440000"), true);
});

test("resolveNotesPeriodeDateDebutIso — periodeId non uuid → InvalidPeriodeIdError", async () => {
  await assert.rejects(
    () => resolveNotesPeriodeDateDebutIso("00000000-0000-4000-8000-000000000001", "trimestre-1"),
    InvalidPeriodeIdError,
  );
});
