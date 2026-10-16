import assert from "node:assert/strict";
import test from "node:test";
import {
  DASHBOARD_SIGNAL_SOURCES,
  runParallelSignalSources,
  unwrapSignalSource,
} from "@/app/lib/dashboard-signals-parallel";

test("runParallelSignalSources — agrège en parallèle et isole les échecs", async () => {
  const order: string[] = [];
  const { outcomes, stepsMs } = await runParallelSignalSources({
    internat: async () => {
      order.push("internat-start");
      await new Promise((r) => setTimeout(r, 30));
      order.push("internat-end");
      return 1;
    },
    rh: async () => {
      order.push("rh-start");
      await new Promise((r) => setTimeout(r, 5));
      order.push("rh-end");
      return 2;
    },
    demandes: async () => {
      throw new Error("demandes down");
    },
    absences: async () => 4,
    documents: async () => 5,
    stages: async () => 6,
    others: async () => 7,
  });

  assert.equal(outcomes.internat.ok && outcomes.internat.value, 1);
  assert.equal(outcomes.rh.ok && outcomes.rh.value, 2);
  assert.equal(outcomes.demandes.ok, false);
  assert.equal(outcomes.absences.ok && outcomes.absences.value, 4);
  assert.deepEqual(unwrapSignalSource(outcomes.demandes, [] as number[]), []);
  assert.equal(DASHBOARD_SIGNAL_SOURCES.length, 7);
  for (const id of DASHBOARD_SIGNAL_SOURCES) {
    assert.ok(typeof stepsMs[id] === "number" && stepsMs[id] >= 0);
  }
  assert.ok(stepsMs.internat >= 25);
  assert.ok(order.includes("internat-start") && order.includes("rh-start"));
});

test("runParallelSignalSources — une source lente ne bloque pas le délai total", async () => {
  const t0 = Date.now();
  await runParallelSignalSources({
    internat: async () => {
      await new Promise((r) => setTimeout(r, 80));
      return null;
    },
    rh: async () => null,
    demandes: async () => null,
    absences: async () => null,
    documents: async () => null,
    stages: async () => null,
    others: async () => null,
  });
  const elapsed = Date.now() - t0;
  assert.ok(elapsed < 200, `attendu parallèle, got ${elapsed}ms`);
});
