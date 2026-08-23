import "server-only";

import { drizzle } from "drizzle-orm/postgres-js";
import postgres from "postgres";
import * as schema from "@/db/schema";

let client: ReturnType<typeof postgres> | null = null;
let dbInstance: ReturnType<typeof drizzle<typeof schema>> | null = null;

export function isDatabaseConfigured(): boolean {
  return Boolean(process.env.DATABASE_URL?.trim());
}

export function getDb() {
  const url = process.env.DATABASE_URL?.trim();
  if (!url) {
    throw new Error("DATABASE_URL manquante — configure PostgreSQL Scaleway.");
  }
  if (!client) {
    client = postgres(url, { max: 10, prepare: false });
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
