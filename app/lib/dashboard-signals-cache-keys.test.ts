import assert from "node:assert/strict";
import test from "node:test";
import {
  dashboardSignalsRolesHash,
  dashboardSignalsScopeHash,
  vsModulesScopeHash,
} from "@/app/lib/dashboard-signals-cache-keys";
import {
  valkeyKeyDashboardSignals,
  valkeyKeyDashboardSignalsAbsenceIndex,
  valkeyKeyDashboardSignalsRequestsBoard,
  valkeyKeyDashboardSignalsRhPlanningNow,
  valkeyKeyDashboardSignalsStagesPending,
  valkeyKeyDashboardSignalsVsCounts,
  valkeyKeyStagesConventionsIndex,
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

test("prof vs direction — clés utilisateur et scope VS distincts", () => {
  const profModules = new Set(["vs-appels", "rh", "mon-planning"]);
  const directionModules = new Set([
    "vs-absences",
    "vs-appels",
    "vs-sanctions",
    "vs-carnet",
    "rh",
    "demandes-hse",
  ]);
  assert.notEqual(vsModulesScopeHash(profModules), vsModulesScopeHash(directionModules));
  assert.notEqual(
    valkeyKeyDashboardSignalsRhPlanningNow("etab-1", "auth-prof"),
    valkeyKeyDashboardSignalsRhPlanningNow("etab-1", "auth-direction"),
  );
  assert.notEqual(
    valkeyKeyDashboardSignals("etab-1", "auth-prof"),
    valkeyKeyDashboardSignals("etab-1", "auth-direction"),
  );
  assert.notEqual(
    valkeyKeyDashboardSignalsStagesPending("etab-1", "auth-prof"),
    valkeyKeyDashboardSignalsRequestsBoard("etab-1", "auth-direction"),
  );
});

test("dashboardSignalsRolesHash — stable et distinct par rôles", () => {
  assert.equal(dashboardSignalsRolesHash(["direction", "enseignant"]), "direction|enseignant");
  assert.notEqual(
    dashboardSignalsRolesHash(["direction"]),
    dashboardSignalsRolesHash(["enseignant"]),
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

  const stageKey = valkeyKeyStagesConventionsIndex("etab-stage");
  let stageLoads = 0;
  for (let i = 0; i < 300; i++) {
    await valkeyCached({
      key: valkeyKeyDashboardSignalsAbsenceIndex(`etab-dash-${i}`),
      ttlSeconds: 30,
      loader: async () => ["x"],
    });
  }
  await valkeyCached({
    key: stageKey,
    ttlSeconds: 30,
    loader: async () => {
      stageLoads += 1;
      return ["stage-index"];
    },
  });
  const stageAgain = await valkeyCached({
    key: stageKey,
    ttlSeconds: 30,
    loader: async () => {
      stageLoads += 1;
      return ["stage-index-2"];
    },
  });
  assert.deepEqual(stageAgain, ["stage-index"]);
  assert.equal(stageLoads, 1);

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
