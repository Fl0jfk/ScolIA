import assert from "node:assert/strict";
import test from "node:test";
import {
  scanPackageJsonTestScripts,
  scanTestFileContent,
} from "./check-test-files-db-safety-core.mjs";

test("scanTestFileContent — refuse postgres() sans garde", () => {
  const content = `
import postgres from "postgres";
const sql = postgres(process.env.TEST_DATABASE_URL);
`;
  const rules = scanTestFileContent("fake/bad.test.ts", content);
  assert.ok(rules.includes("postgres("));
});

test("scanTestFileContent — accepte postgres() avec test-database-guard", () => {
  const content = `
import { resolveTestDatabaseUrl } from "../../scripts/test-database-guard.mjs";
import postgres from "postgres";
const testDb = resolveTestDatabaseUrl();
const sql = postgres(testDb.url, { max: 1 });
`;
  const rules = scanTestFileContent("fake/ok-migration.test.ts", content);
  assert.equal(rules.length, 0);
});

test("scanTestFileContent — refuse dotenv et .env.local", () => {
  const content = `
import dotenv from "dotenv";
dotenv.config({ path: ".env.local" });
`;
  const rules = scanTestFileContent("fake/env.test.ts", content);
  assert.ok(rules.includes("dotenv"));
  assert.ok(rules.includes(".env.local"));
});

test("scanPackageJsonTestScripts — refuse --env-file sur test:*", () => {
  const bad = scanPackageJsonTestScripts({
    "test:unit": "node --env-file=.env.local --test x.test.mjs",
    "build": "next build",
  });
  assert.deepEqual(bad, ["test:unit"]);
});
