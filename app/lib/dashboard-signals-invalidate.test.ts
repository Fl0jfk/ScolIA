import assert from "node:assert/strict";
import test from "node:test";
import {
  invalidateDashboardSignalsAfterAccueilAbsence,
  invalidateDashboardSignalsForEstablishment,
} from "@/app/lib/dashboard-signals-invalidate";

test("invalidateDashboardSignalsAfterAccueilAbsence — no-op si id vide", async () => {
  await invalidateDashboardSignalsAfterAccueilAbsence("", "eleve");
  await invalidateDashboardSignalsAfterAccueilAbsence("   ", "personnel");
  assert.ok(true);
});

test("invalidateDashboardSignalsForEstablishment — no-op si id vide", async () => {
  await invalidateDashboardSignalsForEstablishment("");
  await invalidateDashboardSignalsForEstablishment("   ");
  assert.ok(true);
});
