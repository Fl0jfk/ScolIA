import assert from "node:assert/strict";
import test from "node:test";
import {
  dashboardNotificationBadgeProps,
  isDashboardNotificationUnavailable,
} from "@/app/lib/dashboard-signals-display";
import type { DashboardSignalSourceId, SignalSourceStatus } from "@/app/lib/dashboard-signals-parallel";

const OK = Object.fromEntries(
  ["internat", "rh", "demandes", "absences", "documents", "stages", "others"].map((id) => [
    id,
    "ok" as SignalSourceStatus,
  ]),
) as Record<DashboardSignalSourceId, SignalSourceStatus>;

test("isDashboardNotificationUnavailable — utilise signalSourceId explicite", () => {
  const status = { ...OK, absences: "timeout" as SignalSourceStatus };
  assert.equal(
    isDashboardNotificationUnavailable(
      { id: "x", moduleId: "travels", signalSourceId: "absences" },
      status,
    ),
    true,
  );
  assert.equal(
    isDashboardNotificationUnavailable({ id: "x", moduleId: "travels" }, status),
    false,
  );
});

test("dashboardNotificationBadgeProps — indisponible sans chiffre", () => {
  const status = { ...OK, stages: "error" as SignalSourceStatus };
  assert.deepEqual(
    dashboardNotificationBadgeProps(
      0,
      { id: "stages-pend", moduleId: "stages", signalSourceId: "stages" },
      status,
    ),
    { unavailable: true, compact: undefined },
  );
});

test("dashboardNotificationBadgeProps — null si 0 et source ok", () => {
  assert.equal(
    dashboardNotificationBadgeProps(0, { id: "x", moduleId: "stages", signalSourceId: "stages" }, OK),
    null,
  );
});
