import assert from "node:assert/strict";
import test from "node:test";
import {
  defaultSchoolYearStartIsoFromLabel,
  formatSqlDateToIso,
} from "./notes-periode-debut-logic";
import { isEleveVisiblePourPeriodeNotes } from "./eleve-actif-shared";

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
