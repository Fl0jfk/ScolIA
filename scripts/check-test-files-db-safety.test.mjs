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

test("scanTestFileContent — mention harness en commentaire ne désactive pas postgres()", () => {
  const content = `
// utilise test-database-harness pour les intégrations
import postgres from "postgres";
const sql = postgres(process.env.TEST_DATABASE_URL);
`;
  const rules = scanTestFileContent("fake/comment-only.test.ts", content);
  assert.ok(rules.includes("postgres("));
});

test("scanTestFileContent — refuse comparaison DATABASE_URL", () => {
  const content = `
if (process.env.DATABASE_URL === "postgresql://x") {
  throw new Error("bad");
}
`;
  const rules = scanTestFileContent("fake/compare.test.mts", content);
  assert.ok(rules.includes("DATABASE_URL"));
});

test("scanTestFileContent — refuse SCOLIA_PROD_DATABASE_URL", () => {
  const content = `
const url = process.env.SCOLIA_PROD_DATABASE_URL;
`;
  const rules = scanTestFileContent("fake/prod-url.spec.ts", content);
  assert.ok(rules.includes("DATABASE_URL"));
});

test("scanTestFileContent — import harness autorise postgres()", () => {
  const content = `
import { beginTestDatabase } from "@/app/lib/test-database-harness";
import postgres from "postgres";
const sql = postgres("postgresql://scolia@127.0.0.1:5432/scolia_migrate");
`;
  const rules = scanTestFileContent("fake/harness.test.cjs", content);
  assert.equal(rules.length, 0);
});
