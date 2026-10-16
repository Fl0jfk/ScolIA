import assert from "node:assert/strict";
import test from "node:test";
import {
  dashboardNotificationBadgeProps,
  formatDashboardNotificationCount,
} from "@/app/lib/dashboard-signals-display";
import type { DashboardSignalSourceId, SignalSourceStatus } from "@/app/lib/dashboard-signals-parallel";

const OK = Object.fromEntries(
  ["internat", "rh", "demandes", "absences", "documents", "stages", "others"].map((id) => [
    id,
    "ok" as SignalSourceStatus,
  ]),
) as Record<DashboardSignalSourceId, SignalSourceStatus>;

test("formatDashboardNotificationCount — indisponible si source absences en timeout", () => {
  const status = { ...OK, absences: "timeout" as SignalSourceStatus };
  assert.equal(
    formatDashboardNotificationCount(0, { id: "vs-abs-1", moduleId: "vs-absences" }, status),
    "indisponible",
  );
  assert.equal(
    formatDashboardNotificationCount(3, { id: "vs-abs-1", moduleId: "vs-absences" }, status),
    "indisponible",
  );
});

test("dashboardNotificationBadgeProps — null si 0 et source ok", () => {
  assert.equal(
    dashboardNotificationBadgeProps(0, { id: "x", moduleId: "stages" }, OK),
    null,
  );
});

test("dashboardNotificationBadgeProps — label indisponible", () => {
  const status = { ...OK, stages: "error" as SignalSourceStatus };
  assert.deepEqual(
    dashboardNotificationBadgeProps(0, { id: "stages-pend", moduleId: "stages" }, status),
    { label: "indisponible" },
  );
});
