import assert from "node:assert/strict";
import test from "node:test";
import { buildDashboardSourceStaleAdapters } from "@/app/lib/dashboard-signals-source-stale";

test("sans Valkey, la dernière valeur connue d'une source reste disponible", async () => {
  const a = buildDashboardSourceStaleAdapters("etab", "user-1", "r1").rh!;
  assert.equal(await a.readStale(), null);
  await a.writeFresh({ n: 3 });
  assert.deepEqual(await a.readStale(), { n: 3 });
});

test("la valeur de secours n'est jamais partagée entre deux utilisateurs", async () => {
  const a = buildDashboardSourceStaleAdapters("etab", "user-2", "r1").absences!;
  const b = buildDashboardSourceStaleAdapters("etab", "user-3", "r1").absences!;
  await a.writeFresh({ n: 9 });
  assert.equal(await b.readStale(), null);
});
