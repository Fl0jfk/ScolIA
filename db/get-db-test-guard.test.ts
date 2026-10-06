/**
 * Garde runtime getDb() en SCOLIA_TEST_MODE — refuse une URL prod-like.
 */
import assert from "node:assert/strict";
import test from "node:test";

test("getDb — refuse hôte Scaleway quand SCOLIA_TEST_MODE=1", async () => {
  process.env.SCOLIA_TEST_MODE = "1";
  process.env.DATABASE_URL =
    "postgresql://user:pass@rdb.fr-par.scw.cloud:5432/scolia_migrate";

  const { closeDb, getDb } = await import("@/db/index");
  await closeDb();

  assert.throws(
    () => getDb(),
    (err: unknown) => {
      assert.ok(err instanceof Error);
      assert.match(err.message, /db-guard|refus/i);
      return true;
    },
  );

  delete process.env.SCOLIA_TEST_MODE;
  delete process.env.DATABASE_URL;
  await closeDb();
});
