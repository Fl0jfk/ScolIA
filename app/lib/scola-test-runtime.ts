import { isTestProcess } from "../../scripts/test-database-guard.mjs";

/** node:test ou npm script `SCOLIA_TEST_MODE=1`, jamais en production. */
export function isNonProductionTestProcess(): boolean {
  return isTestProcess() && process.env.NODE_ENV !== "production";
}

/** Harness PGlite stages (`SCOLA_TEST_DB=1` + test process non-prod). */
export function isPgliteIntegrationTest(): boolean {
  return isNonProductionTestProcess() && process.env.SCOLA_TEST_DB === "1";
}
