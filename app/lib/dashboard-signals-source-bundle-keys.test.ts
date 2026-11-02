import assert from "node:assert/strict";
import test from "node:test";
import { dashboardSignalsRolesHash } from "@/app/lib/dashboard-signals-cache-keys";
import { valkeyKeyDashboardSignalSourceBundle } from "@/app/lib/dashboard-signals-source-stale";

test("valkeyKeyDashboardSignalSourceBundle — inclut hash rôles", () => {
  const rolesA = dashboardSignalsRolesHash(["direction"]);
  const rolesB = dashboardSignalsRolesHash(["enseignant"]);
  const keyA = valkeyKeyDashboardSignalSourceBundle("etab-1", "user-1", rolesA, "rh");
  const keyB = valkeyKeyDashboardSignalSourceBundle("etab-1", "user-1", rolesB, "rh");
  assert.notEqual(keyA, keyB);
  assert.match(keyA, /:direction:rh$/);
});
