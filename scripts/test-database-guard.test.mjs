import assert from "node:assert/strict";
import test from "node:test";
import { validateTestDatabaseUrl } from "./test-database-guard.mjs";

test("validateTestDatabaseUrl — accepte CI / local test", () => {
  const ok = validateTestDatabaseUrl("postgresql://scolia@127.0.0.1:5432/scolia_migrate");
  assert.equal(ok.ok, true);
});

test("validateTestDatabaseUrl — refuse prod scaleway", () => {
  const bad = validateTestDatabaseUrl(
    "postgresql://user:pass@rdb.fr-par.scw.cloud:5432/scola",
  );
  assert.equal(bad.ok, false);
});

test("validateTestDatabaseUrl — refuse base dev scola sans suffix test", () => {
  const bad = validateTestDatabaseUrl("postgresql://scola@127.0.0.1:5432/scola");
  assert.equal(bad.ok, false);
});
