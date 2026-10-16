import assert from "node:assert/strict";
import test from "node:test";
import { runParallelSignalSources } from "@/app/lib/dashboard-signals-parallel";

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
    { timeoutMs: 3_000 },
  );
  assert.equal(sourceStatus.internat, "ok");
  assert.equal(outcomes.internat.ok && outcomes.internat.value, 1);
});

test("runParallelSignalSources — timeout puis poll suivant avec cache stale", async () => {
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
    { timeoutMs: 50, stale: { internat: stale } },
  );
  assert.equal(first.sourceStatus.internat, "timeout");

  store = 99;
  const second = await runParallelSignalSources(
    { ...NULL_LOADERS, internat: slowInternat },
    { timeoutMs: 50, stale: { internat: stale } },
  );
  assert.equal(second.sourceStatus.internat, "stale");
  assert.equal(second.outcomes.internat.ok && second.outcomes.internat.value, 99);

  const third = await runParallelSignalSources(
    {
      ...NULL_LOADERS,
      internat: async () => 7,
    },
    { timeoutMs: 500, stale: { internat: stale } },
  );
  assert.equal(third.sourceStatus.internat, "ok");
  assert.equal(third.outcomes.internat.ok && third.outcomes.internat.value, 7);
  assert.equal(store, 7);
});
