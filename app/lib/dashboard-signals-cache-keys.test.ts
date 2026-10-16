import assert from "node:assert/strict";
import test from "node:test";
import { dashboardSignalsScopeHash, vsModulesScopeHash } from "@/app/lib/dashboard-signals-cache-keys";
import {
  valkeyKeyDashboardSignals,
  valkeyKeyDashboardSignalsAbsenceIndex,
  valkeyKeyDashboardSignalsRequestsBoard,
  valkeyKeyDashboardSignalsRhPlanningNow,
  valkeyKeyDashboardSignalsStagesPending,
  valkeyKeyDashboardSignalsVsCounts,
} from "@/app/lib/valkey-keys";
import { isScopedMemoryCacheKey, resetValkeyMemoryCacheForTests, valkeyCached } from "@/app/lib/valkey";

test("clés cache signaux — isolation établissement et utilisateur", () => {
  assert.notEqual(
    valkeyKeyDashboardSignals("etab-a", "user-1"),
    valkeyKeyDashboardSignals("etab-b", "user-1"),
  );
  assert.notEqual(
    valkeyKeyDashboardSignals("etab-a", "user-1"),
    valkeyKeyDashboardSignals("etab-a", "user-2"),
  );
  assert.notEqual(
    valkeyKeyDashboardSignalsRhPlanningNow("etab-a", "u1"),
    valkeyKeyDashboardSignalsRhPlanningNow("etab-a", "u2"),
  );
  assert.notEqual(
    valkeyKeyDashboardSignalsStagesPending("etab-a", "u1"),
    valkeyKeyDashboardSignalsRequestsBoard("etab-a", "u2"),
  );
});

test("vsModulesScopeHash — diffère selon modules VS visibles", () => {
  const direction = new Set(["vs-absences", "vs-appels", "vs-sanctions"]);
  const teacher = new Set(["vs-appels"]);
  assert.notEqual(vsModulesScopeHash(direction), vsModulesScopeHash(teacher));
  assert.equal(
    vsModulesScopeHash(new Set(["vs-carnet", "vs-absences"])),
    dashboardSignalsScopeHash(["vs-absences", "vs-carnet"]),
  );
});

test("valkeyCached dash:sig-src — pas de fuite entre établissements sans Valkey", async () => {
  process.env.VALKEY_DISABLED = "1";
  resetValkeyMemoryCacheForTests();
  let runsA = 0;
  let runsB = 0;
  const keyA = valkeyKeyDashboardSignalsAbsenceIndex("etab-a");
  const keyB = valkeyKeyDashboardSignalsAbsenceIndex("etab-b");
  assert.equal(isScopedMemoryCacheKey(keyA), true);
  assert.equal(isScopedMemoryCacheKey(valkeyKeyDashboardSignalsVsCounts("e1", "vs-absences")), true);

  await valkeyCached({
    key: keyA,
    ttlSeconds: 30,
    loader: async () => {
      runsA += 1;
      return ["a"];
    },
  });
  await valkeyCached({
    key: keyB,
    ttlSeconds: 30,
    loader: async () => {
      runsB += 1;
      return ["b"];
    },
  });
  const againA = await valkeyCached({
    key: keyA,
    ttlSeconds: 30,
    loader: async () => {
      runsA += 1;
      return ["a2"];
    },
  });
  assert.deepEqual(againA, ["a"]);
  assert.equal(runsA, 1);
  assert.equal(runsB, 1);
});
