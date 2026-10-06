/**
 * Harness tests d’intégration Postgres — TEST_DATABASE_URL uniquement.
 */
import assert from "node:assert/strict";
import type { TestContext } from "node:test";
import {
  installTestDatabaseUrlForApp,
  resolveTestDatabaseUrl,
  testDatabaseSkipReason,
} from "../../scripts/test-database-guard.mjs";

export type ValidatedTestDatabase = { ok: true; url: string };

export function beginTestDatabase(t: TestContext): ValidatedTestDatabase | null {
  const result = resolveTestDatabaseUrl();
  const skip = testDatabaseSkipReason(result);
  if (skip || !result.ok) {
    const reason = skip ?? "TEST_DATABASE_URL refusée";
    if (process.env.CI === "true") {
      assert.fail(reason);
    }
    t.skip(reason);
    return null;
  }
  installTestDatabaseUrlForApp(result);
  return result;
}

export async function endTestDatabase(closeDb?: () => Promise<void>): Promise<void> {
  if (closeDb) {
    await closeDb();
  }
  delete process.env.DATABASE_URL;
}
