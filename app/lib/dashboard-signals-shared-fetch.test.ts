import assert from "node:assert/strict";
import test from "node:test";
import {
  fetchDashboardSignalsShared,
  normalizeDashboardSignalsJson,
  resetDashboardSignalsSharedFetch,
} from "@/app/lib/dashboard-signals-shared-fetch";

const OK_STATUS = Object.fromEntries(
  ["internat", "rh", "demandes", "absences", "documents", "stages", "others"].map((id) => [
    id,
    "ok",
  ]),
);

test("fetchDashboardSignalsShared — ignore résultat si session user a changé", async () => {
  resetDashboardSignalsSharedFetch();
  let resolveSlow: (v: unknown) => void = () => undefined;
  const slow = new Promise<unknown>((r) => {
    resolveSlow = r;
  });

  const p1 = fetchDashboardSignalsShared("user-a", {
    fetchJson: () => slow,
  });

  resetDashboardSignalsSharedFetch();

  resolveSlow({
    shortcuts: [],
    todayNews: [],
    hasCurrentWeek: false,
    notifications: [{ id: "n1" }],
    perf: { sourceStatus: OK_STATUS },
  });

  const r1 = await p1;
  assert.equal(r1.payload, null);
  assert.equal(r1.wroteClientCache, false);

  const p2 = await fetchDashboardSignalsShared("user-b", {
    fetchJson: async () => ({
      shortcuts: [{ id: "s1" }],
      todayNews: [],
      hasCurrentWeek: true,
      notifications: [],
      perf: { sourceStatus: OK_STATUS },
    }),
  });
  assert.equal(p2.payload?.shortcuts.length, 1);
});

test("fetchDashboardSignalsShared — pas de cache client si source en timeout", async () => {
  resetDashboardSignalsSharedFetch();
  const r = await fetchDashboardSignalsShared("user-cache", {
    fetchJson: async () => ({
      shortcuts: [],
      todayNews: [],
      hasCurrentWeek: false,
      notifications: [],
      perf: {
        sourceStatus: { ...OK_STATUS, demandes: "timeout" },
      },
    }),
  });
  assert.ok(r.payload);
  assert.equal(r.wroteClientCache, false);
});

test("normalizeDashboardSignalsJson — extrait le payload métier", () => {
  const s = normalizeDashboardSignalsJson({
    shortcuts: [],
    todayNews: [],
    hasCurrentWeek: true,
    notifications: [],
    perf: { sourceStatus: OK_STATUS },
  });
  assert.equal(s.hasCurrentWeek, true);
});
