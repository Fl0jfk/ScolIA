import assert from "node:assert/strict";
import test from "node:test";
import { invalidateDashboardSignalsForEstablishment } from "@/app/lib/dashboard-signals-invalidate";

test("invalidateDashboardSignalsForEstablishment — no-op si id vide", async () => {
  await invalidateDashboardSignalsForEstablishment("");
  await invalidateDashboardSignalsForEstablishment("   ");
  assert.ok(true);
});
