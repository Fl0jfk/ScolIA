import assert from "node:assert/strict";
import test from "node:test";
import { resolveDashboardSignalSource } from "@/app/lib/dashboard-signal-source";

test("resolveDashboardSignalSource — module absences → source absences (pas rh)", () => {
  assert.equal(resolveDashboardSignalSource({ moduleId: "absences" }), "absences");
  assert.equal(
    resolveDashboardSignalSource({ id: "absences-pending", moduleId: "absences" }),
    "absences",
  );
});

test("resolveDashboardSignalSource — mon-planning reste sur rh", () => {
  assert.equal(resolveDashboardSignalSource({ moduleId: "mon-planning" }), "rh");
});
