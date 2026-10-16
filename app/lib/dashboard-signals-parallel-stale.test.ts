import assert from "node:assert/strict";
import test from "node:test";
import { isDashboardSignalsAggregateCacheable } from "@/app/lib/dashboard-signals-aggregate-cache";
import { runParallelSignalSources } from "@/app/lib/dashboard-signals-parallel";
import {
  resetDashboardSignalSourceFlightForTests,
  runDashboardSignalSourceFlight,
} from "@/app/lib/dashboard-signals-source-flight";

const NULL_LOADERS = {
  internat: async () => 0,
  rh: async () => 0,
  demandes: async () => 0,
  absences: async () => 0,
  documents: async () => 0,
  stages: async () => 0,
  others: async () => 0,
};

test("runParallelSignalSources — source lente (2s) reste ok avec timeout 3s", async () => {
  const { outcomes, sourceStatus } = await runParallelSignalSources(
    {
      ...NULL_LOADERS,
      internat: async () => {
        await new Promise((r) => setTimeout(r, 2_000));
        return 1;
      },
    },
    { timeoutMs: 3_000, softTimeoutMs: 100 },
  );
  assert.equal(sourceStatus.internat, "ok");
  assert.equal(outcomes.internat.ok && outcomes.internat.value, 1);
});

test("runParallelSignalSources — stale au soft budget, load unique remplit le cache", async () => {
  let store: number | null = 99;
  let loadCalls = 0;
  const slowInternat = async () => {
    loadCalls += 1;
    await new Promise((r) => setTimeout(r, 200));
    return 42;
  };
  const stale = {
    readStale: async () => store,
    writeFresh: async (v: unknown) => {
      store = v as number;
    },
  };

  const t0 = Date.now();
  const first = await runParallelSignalSources(
    { ...NULL_LOADERS, internat: slowInternat },
    { timeoutMs: 500, softTimeoutMs: 50, stale: { internat: stale }, flightKeys: { internat: "flight-internat" } },
  );
  const elapsed = Date.now() - t0;

  assert.equal(first.sourceStatus.internat, "stale");
  assert.equal(first.outcomes.internat.ok && first.outcomes.internat.value, 99);
  assert.ok(elapsed < 150, `attendu réponse rapide stale, got ${elapsed}ms`);
  assert.equal(loadCalls, 1);

  await new Promise((r) => setTimeout(r, 250));
  assert.equal(store, 42);
  assert.equal(loadCalls, 1);
});

test("runParallelSignalSources — cold sans stale attend jusqu’au hard timeout", async () => {
  let store: number | null = null;
  const slowInternat = async () => {
    await new Promise((r) => setTimeout(r, 200));
    return 42;
  };
  const stale = {
    readStale: async () => store,
    writeFresh: async (v: unknown) => {
      store = v as number;
    },
  };

  const first = await runParallelSignalSources(
    { ...NULL_LOADERS, internat: slowInternat },
    { timeoutMs: 50, softTimeoutMs: 20, stale: { internat: stale } },
  );
  assert.equal(first.sourceStatus.internat, "timeout");

  store = 99;
  const second = await runParallelSignalSources(
    { ...NULL_LOADERS, internat: slowInternat },
    { timeoutMs: 50, softTimeoutMs: 20, stale: { internat: stale } },
  );
  assert.equal(second.sourceStatus.internat, "stale");
  assert.equal(second.outcomes.internat.ok && second.outcomes.internat.value, 99);

  const third = await runParallelSignalSources(
    {
      ...NULL_LOADERS,
      internat: async () => 7,
    },
    { timeoutMs: 500, softTimeoutMs: 50, stale: { internat: stale } },
  );
  assert.equal(third.sourceStatus.internat, "ok");
  assert.equal(third.outcomes.internat.ok && third.outcomes.internat.value, 7);
  assert.equal(store, 7);
});

test("runDashboardSignalSourceFlight — 3 appels concurrents = 1 loader", async () => {
  resetDashboardSignalSourceFlightForTests();
  let calls = 0;
  const loader = async () => {
    calls += 1;
    await new Promise((r) => setTimeout(r, 30));
    return "x";
  };
  const results = await Promise.all([
    runDashboardSignalSourceFlight("k1", loader),
    runDashboardSignalSourceFlight("k1", loader),
    runDashboardSignalSourceFlight("k1", loader),
  ]);
  assert.deepEqual(results, ["x", "x", "x"]);
  assert.equal(calls, 1);
});

test("isDashboardSignalsAggregateCacheable — refuse agrégat si une source stale", () => {
  const status = {
    internat: "ok",
    rh: "ok",
    demandes: "ok",
    absences: "ok",
    documents: "ok",
    stages: "ok",
    others: "stale",
  } as const;
  assert.equal(isDashboardSignalsAggregateCacheable(status), false);
});
