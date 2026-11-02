import "server-only";

import { drizzle } from "drizzle-orm/postgres-js";
import postgres from "postgres";
import { assertDatabaseUrlAllowedInTestProcess } from "../scripts/test-database-guard.mjs";
import { isPgliteIntegrationTest } from "@/app/lib/scola-test-runtime";
import * as schema from "@/db/schema";

let client: ReturnType<typeof postgres> | null = null;
let dbInstance: ReturnType<typeof drizzle<typeof schema>> | null = null;
/** Connexion PGlite injectée par les tests — jamais mélangée au client postgres réel. */
let integrationTestDb: ReturnType<typeof drizzle<typeof schema>> | null = null;

export function isDatabaseConfigured(): boolean {
  if (isPgliteIntegrationTest() && integrationTestDb) return true;
  return Boolean(process.env.DATABASE_URL?.trim());
}

/** PGlite uniquement — voir `test:stages-perf`. */
export function setIntegrationTestDb(db: Db): void {
  if (!isPgliteIntegrationTest()) {
    throw new Error("setIntegrationTestDb: SCOLA_TEST_DB=1 + processus de test non-prod requis");
  }
  if (client) {
    throw new Error("setIntegrationTestDb: client PostgreSQL déjà initialisé");
  }
  integrationTestDb = db;
}

export function getDb() {
  if (isPgliteIntegrationTest() && integrationTestDb) {
    return integrationTestDb;
  }
  const url = process.env.DATABASE_URL?.trim();
  if (!url) {
    throw new Error("DATABASE_URL manquante — configure PostgreSQL Scaleway.");
  }
  assertDatabaseUrlAllowedInTestProcess(url);
  if (!client) {
    client = postgres(url, {
      // max_scale=3 → ~24 connexions app ; Postgres Scaleway = 100.
      max: 8,
      prepare: false,
      idle_timeout: 20,
      max_lifetime: 60 * 10,
      connect_timeout: 3,
    });
    dbInstance = drizzle(client, { schema });
  }
  return dbInstance!;
}

export async function closeDb(): Promise<void> {
  if (integrationTestDb) {
    integrationTestDb = null;
    return;
  }
  if (client) {
    await client.end({ timeout: 5 });
    client = null;
    dbInstance = null;
  }
}

export type Db = ReturnType<typeof getDb>;
