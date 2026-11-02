import assert from "node:assert/strict";
import test from "node:test";
import {
  LEGACY_MIGRATION_ERROR_BACKOFF_MS,
  armLegacyMigrationBackoffForTests,
  getLegacyMigrationBackoffUntilForTests,
  resetConventionsLegacyMigrationMemoryForTests,
} from "@/app/lib/stage-db";

process.env.SCOLIA_TEST_MODE = "1";

test("migration legacy — backoff fixé à 60 s", () => {
  assert.equal(LEGACY_MIGRATION_ERROR_BACKOFF_MS, 60_000);
});

test("migration legacy — armLegacyMigrationBackoffForTests pose une fenêtre ~60 s", () => {
  resetConventionsLegacyMigrationMemoryForTests();
  const etab = "etab-backoff-test";
  const t0 = Date.now();
  armLegacyMigrationBackoffForTests(etab);
  const until = getLegacyMigrationBackoffUntilForTests(etab);
  assert.ok(until !== undefined);
  assert.ok(until >= t0 + 59_000);
  assert.ok(until <= t0 + 61_000);
});
