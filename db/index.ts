import "server-only";

import { drizzle } from "drizzle-orm/postgres-js";
import postgres from "postgres";
import { assertDatabaseUrlAllowedInTestProcess, isTestProcess } from "../scripts/test-database-guard.mjs";
import * as schema from "@/db/schema";

let client: ReturnType<typeof postgres> | null = null;
let dbInstance: ReturnType<typeof drizzle<typeof schema>> | null = null;

/** Journal SQL en tests (SCOLIA_TEST_MODE) — compter les requêtes par flux. */
let testSqlLog: string[] = [];

export function resetTestSqlLog(): void {
  testSqlLog = [];
}

export function drainTestSqlLog(): string[] {
  const copy = testSqlLog.slice();
  testSqlLog = [];
  return copy;
}

export function isDatabaseConfigured(): boolean {
  return Boolean(process.env.DATABASE_URL?.trim());
}

export function getDb() {
  const url = process.env.DATABASE_URL?.trim();
  if (!url) {
    throw new Error("DATABASE_URL manquante — configure PostgreSQL Scaleway.");
  }
  assertDatabaseUrlAllowedInTestProcess(url);
  if (!client) {
    const logSqlInTest = isTestProcess();
    client = postgres(url, {
      // max_scale=3 → ~24 connexions app ; Postgres Scaleway = 100.
      max: 8,
      prepare: false,
      idle_timeout: 20,
      max_lifetime: 60 * 10,
      connect_timeout: 3,
      ...(logSqlInTest
        ? {
            debug: (_connection: number, query: string) => {
              testSqlLog.push(query);
            },
          }
        : {}),
    });
    dbInstance = drizzle(client, { schema });
  }
  return dbInstance!;
}

export async function closeDb(): Promise<void> {
  if (client) {
    await client.end({ timeout: 5 });
    client = null;
    dbInstance = null;
  }
}

export type Db = ReturnType<typeof getDb>;
