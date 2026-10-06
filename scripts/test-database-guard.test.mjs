import assert from "node:assert/strict";
import test from "node:test";
import {
  assertDatabaseUrlAllowedInTestProcess,
  isTestProcess,
  validateDevScriptDatabaseUrl,
  validateTestDatabaseUrl,
} from "./test-database-guard.mjs";

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

test("validateTestDatabaseUrl — refuse nom contenant _migrate sans suffixe _migrate", () => {
  const bad = validateTestDatabaseUrl("postgresql://scola@127.0.0.1:5432/scola_migrate_prod");
  assert.equal(bad.ok, false);
});

test("validateDevScriptDatabaseUrl — accepte scola local", () => {
  const prev = process.env.I_KNOW_THIS_IS_NOT_PROD;
  delete process.env.I_KNOW_THIS_IS_NOT_PROD;
  const ok = validateDevScriptDatabaseUrl("postgresql://scola@127.0.0.1:5432/scola");
  assert.equal(ok.ok, true);
  if (prev !== undefined) process.env.I_KNOW_THIS_IS_NOT_PROD = prev;
});

test("validateDevScriptDatabaseUrl — refuse hôte distant", () => {
  const prev = process.env.I_KNOW_THIS_IS_NOT_PROD;
  delete process.env.I_KNOW_THIS_IS_NOT_PROD;
  const bad = validateDevScriptDatabaseUrl(
    "postgresql://user:pass@rdb.fr-par.scw.cloud:5432/scolia_migrate",
  );
  assert.equal(bad.ok, false);
  if (prev !== undefined) process.env.I_KNOW_THIS_IS_NOT_PROD = prev;
});

test("validateDevScriptDatabaseUrl — I_KNOW_THIS_IS_NOT_PROD ne contourne pas l’hôte distant", () => {
  const prev = process.env.I_KNOW_THIS_IS_NOT_PROD;
  process.env.I_KNOW_THIS_IS_NOT_PROD = "1";
  const bad = validateDevScriptDatabaseUrl(
    "postgresql://user:pass@rdb.fr-par.scw.cloud:5432/scolia_migrate",
  );
  assert.equal(bad.ok, false);
  if (prev !== undefined) process.env.I_KNOW_THIS_IS_NOT_PROD = prev;
  else delete process.env.I_KNOW_THIS_IS_NOT_PROD;
});

test("validateDevScriptDatabaseUrl — accepte [::1] local", () => {
  const prev = process.env.I_KNOW_THIS_IS_NOT_PROD;
  delete process.env.I_KNOW_THIS_IS_NOT_PROD;
  const ok = validateDevScriptDatabaseUrl("postgresql://scola@[::1]:5432/scola");
  assert.equal(ok.ok, true);
  if (prev !== undefined) process.env.I_KNOW_THIS_IS_NOT_PROD = prev;
});

test("validateDevScriptDatabaseUrl — I_KNOW_THIS_IS_NOT_PROD relaxe le suffixe en local", () => {
  const prev = process.env.I_KNOW_THIS_IS_NOT_PROD;
  process.env.I_KNOW_THIS_IS_NOT_PROD = "1";
  const ok = validateDevScriptDatabaseUrl("postgresql://scola@127.0.0.1:5432/custom_lab_db");
  assert.equal(ok.ok, true);
  if (prev !== undefined) process.env.I_KNOW_THIS_IS_NOT_PROD = prev;
  else delete process.env.I_KNOW_THIS_IS_NOT_PROD;
});

test("assertDatabaseUrlAllowedInTestProcess — actif si SCOLIA_TEST_MODE", () => {
  const prevMode = process.env.SCOLIA_TEST_MODE;
  process.env.SCOLIA_TEST_MODE = "1";
  assert.equal(isTestProcess(), true);
  assert.throws(() => {
    assertDatabaseUrlAllowedInTestProcess("postgresql://scola@127.0.0.1:5432/scola");
  });
  if (prevMode !== undefined) process.env.SCOLIA_TEST_MODE = prevMode;
  else delete process.env.SCOLIA_TEST_MODE;
});
