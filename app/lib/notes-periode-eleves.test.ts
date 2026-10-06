import assert from "node:assert/strict";
import test from "node:test";
import { isEleveVisiblePourPeriodeNotes } from "./eleve-actif-shared";

test("visibilité notes : sortie après début de période → inclus", () => {
  assert.equal(isEleveVisiblePourPeriodeNotes("2026-03-15", "2026-01-01"), true);
});

test("visibilité notes : sortie avant début de période → exclu", () => {
  assert.equal(isEleveVisiblePourPeriodeNotes("2026-02-15", "2026-04-01"), false);
});

test("visibilité notes : sans date de sortie → inclus", () => {
  assert.equal(isEleveVisiblePourPeriodeNotes(null, "2026-04-01"), true);
});
