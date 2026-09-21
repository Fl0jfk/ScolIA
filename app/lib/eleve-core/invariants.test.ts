import assert from "node:assert/strict";
import test from "node:test";
import {
  assertDifferentYears,
  assertNotTwoClassesSameYear,
  compareIsoDate,
  dayBefore,
  EleveCoreError,
  nextSchoolYearLabelFrom,
  planRegimeCutover,
  regimeAtDate,
  regimeImpliesDemiPension,
  storedRegimeLabel,
} from "./invariants";

test("dayBefore — 15 janvier → 14 janvier", () => {
  assert.equal(dayBefore("2026-01-15"), "2026-01-14");
  assert.equal(dayBefore("2026-03-01"), "2026-02-28");
});

test("nextSchoolYearLabelFrom", () => {
  assert.equal(nextSchoolYearLabelFrom("2025-2026"), "2026-2027");
});

test("planRegimeCutover — DP puis interne au 15 janvier", () => {
  const plan = planRegimeCutover({
    open: { regime: "Demi-pension", dateDebut: "2025-09-01", dateFin: null },
    nextRegime: "3",
    effectiveOn: "2026-01-15",
  });
  assert.equal(plan.noop, false);
  assert.equal(plan.closeDateFin, "2026-01-14");
  assert.equal(plan.next.regime, "Interne");
  assert.equal(plan.next.dateDebut, "2026-01-15");
  assert.equal(plan.next.dateFin, null);
});

test("planRegimeCutover — même régime = no-op", () => {
  const plan = planRegimeCutover({
    open: { regime: "Interne", dateDebut: "2025-09-01", dateFin: null },
    nextRegime: "interne",
    effectiveOn: "2026-01-15",
  });
  assert.equal(plan.noop, true);
});

test("planRegimeCutover — date d’effet trop tôt", () => {
  assert.throws(
    () =>
      planRegimeCutover({
        open: { regime: "Externe", dateDebut: "2025-09-01", dateFin: null },
        nextRegime: "Demi-pension",
        effectiveOn: "2025-09-01",
      }),
    (err: unknown) => err instanceof EleveCoreError && err.code === "REGIME_DATE_ORDER",
  );
});

test("regimeAtDate — facture lit DP jusqu’au 14, interne dès le 15", () => {
  const periods = [
    { regime: "Demi-pension", dateDebut: "2025-09-01", dateFin: "2026-01-14" },
    { regime: "Interne", dateDebut: "2026-01-15", dateFin: null },
  ];
  assert.equal(regimeAtDate(periods, "2026-01-14"), "Demi-pension");
  assert.equal(regimeAtDate(periods, "2026-01-15"), "Interne");
  assert.equal(regimeAtDate(periods, "2025-12-01"), "Demi-pension");
});

test("assertNotTwoClassesSameYear", () => {
  assert.throws(
    () =>
      assertNotTwoClassesSameYear({
        existingActiveSameYearId: "aaa",
        incomingId: "bbb",
      }),
    (err: unknown) => err instanceof EleveCoreError && err.code === "TWO_CLASSES_SAME_YEAR",
  );
  assert.doesNotThrow(() =>
    assertNotTwoClassesSameYear({ existingActiveSameYearId: "aaa", incomingId: "aaa" }),
  );
});

test("assertDifferentYears — prevue ≠ année en cours", () => {
  assert.throws(
    () => assertDifferentYears("year-1", "year-1"),
    (err: unknown) => err instanceof EleveCoreError && err.code === "SAME_YEAR",
  );
  assert.doesNotThrow(() => assertDifferentYears("year-1", "year-2"));
});

test("storedRegimeLabel + demiPension", () => {
  assert.equal(storedRegimeLabel("2"), "Demi-pension");
  assert.equal(regimeImpliesDemiPension("2"), true);
  assert.equal(regimeImpliesDemiPension("3"), false);
  assert.equal(compareIsoDate("2026-01-14", "2026-01-15") < 0, true);
});
