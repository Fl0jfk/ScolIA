import assert from "node:assert/strict";
import test from "node:test";
import { isDashboardSignalsAggregateCacheable } from "@/app/lib/dashboard-signals-aggregate-cache";
import {
  DASHBOARD_SIGNAL_SOURCES,
  type DashboardSignalSourceId,
  type SignalSourceStatus,
} from "@/app/lib/dashboard-signals-parallel";

function allOk(): Record<DashboardSignalSourceId, SignalSourceStatus> {
  return Object.fromEntries(DASHBOARD_SIGNAL_SOURCES.map((id) => [id, "ok"])) as Record<
    DashboardSignalSourceId,
    SignalSourceStatus
  >;
}

test("isDashboardSignalsAggregateCacheable — ok si toutes les sources ok", () => {
  assert.equal(isDashboardSignalsAggregateCacheable(allOk()), true);
});

test("isDashboardSignalsAggregateCacheable — refuse timeout ou error", () => {
  const partial: Record<DashboardSignalSourceId, SignalSourceStatus> = {
    ...allOk(),
    others: "timeout",
  };
  assert.equal(isDashboardSignalsAggregateCacheable(partial), false);
  const err: Record<DashboardSignalSourceId, SignalSourceStatus> = { ...allOk(), rh: "error" };
  assert.equal(isDashboardSignalsAggregateCacheable(err), false);
  assert.equal(isDashboardSignalsAggregateCacheable(undefined), false);
});
