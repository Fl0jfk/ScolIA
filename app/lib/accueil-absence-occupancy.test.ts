/**
 * Tests purs — garde-fou accueil × occupancy.
 */

import { test } from "node:test";
import assert from "node:assert/strict";
import {
  ACCUEIL_EN_SORTIE_CODE,
  AccueilEnSortieConflictError,
  formatAccueilEnSortieConfirm,
  formatAccueilEnSortieWarning,
  periodDatesForAccueilCheck,
  picksEnSortieHits,
} from "./accueil-absence-occupancy";

test("picksEnSortieHits — ignore absent_vs / en_cours", () => {
  const hits = picksEnSortieHits([
    { date: "2026-09-21", tag: "absent_vs", source: "vs:1" },
    {
      date: "2026-09-21",
      tag: "en_sortie",
      source: "travel:t1",
      detail: { travelId: "t1" },
    },
    { date: "2026-09-22", tag: "en_cours", source: "default" },
  ]);
  assert.equal(hits.length, 1);
  assert.equal(hits[0]!.tag, "en_sortie");
  assert.equal(hits[0]!.travelId, "t1");
});

test("formatAccueilEnSortieWarning — sortie ≠ absence bulletin", () => {
  const msg = formatAccueilEnSortieWarning([
    { date: "2026-09-21", tag: "en_sortie", travelId: "t1" },
  ]);
  assert.match(msg, /sortie scolaire/);
  assert.match(msg, /pas une absence bulletin/i);
  assert.match(msg, /2026-09-21/);
});

test("formatAccueilEnSortieConfirm — invite au force", () => {
  const msg = formatAccueilEnSortieConfirm([
    { date: "2026-09-21", tag: "en_sortie" },
  ]);
  assert.match(msg, /Forcer/);
});

test("periodDatesForAccueilCheck — inclusive", () => {
  assert.deepEqual(periodDatesForAccueilCheck("2026-09-21", "2026-09-21"), ["2026-09-21"]);
  assert.deepEqual(periodDatesForAccueilCheck("2026-09-21", "2026-09-23"), [
    "2026-09-21",
    "2026-09-22",
    "2026-09-23",
  ]);
});

test("AccueilEnSortieConflictError — code stable", () => {
  const err = new AccueilEnSortieConflictError([
    { date: "2026-09-21", tag: "en_sortie", travelId: "x" },
  ]);
  assert.equal(err.code, ACCUEIL_EN_SORTIE_CODE);
  assert.equal(err.hits.length, 1);
});
