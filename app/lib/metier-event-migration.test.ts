/**
 * Vérifie la migration 0060_metier_event (schéma prod LPNB manquant).
 * npx tsx --test app/lib/metier-event-migration.test.ts
 */
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";

function loadEnvFile(filePath: string) {
  if (!existsSync(filePath)) return;
  for (const line of readFileSync(filePath, "utf8").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const eqIdx = trimmed.indexOf("=");
    if (eqIdx <= 0) continue;
    const key = trimmed.slice(0, eqIdx);
    let value = trimmed.slice(eqIdx + 1);
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }
    if (!(key in process.env)) process.env[key] = value;
  }
}

loadEnvFile(".env.local");
loadEnvFile(".env");

function splitMigrationStatements(fileContent: string): string[] {
  return fileContent
    .split(/--> statement-breakpoint\n?/g)
    .map((s) => s.trim())
    .filter(Boolean);
}

const hasDb = Boolean(process.env.DATABASE_URL?.trim());

test("0060_metier_event — CREATE idempotent (simulation prod sans table)", async (t) => {
  if (!hasDb) {
    t.skip("DATABASE_URL absente");
    return;
  }

  const migrationPath = path.join(process.cwd(), "drizzle", "0060_metier_event.sql");
  const statements = splitMigrationStatements(readFileSync(migrationPath, "utf8"));
  assert.ok(statements.length >= 4);

  const postgres = (await import("postgres")).default;
  const url = process.env.DATABASE_URL!;
  const sql = postgres(url, { max: 1 });

  try {
    await sql`DROP TABLE IF EXISTS metier_event CASCADE`;
    const [missing] = await sql`SELECT to_regclass('public.metier_event')::text AS reg`;
    assert.equal(missing.reg, null);

    for (const stmt of statements) {
      await sql.unsafe(stmt);
    }
    for (const stmt of statements) {
      await sql.unsafe(stmt);
    }

    const [present] = await sql`SELECT to_regclass('public.metier_event')::text AS reg`;
    assert.equal(present.reg, "metier_event");

    const indexes = await sql`
      SELECT indexname FROM pg_indexes
      WHERE schemaname = 'public' AND tablename = 'metier_event'
    `;
    const names = new Set(indexes.map((r) => r.indexname as string));
    assert.ok(names.has("metier_event_etab_created_idx"));
    assert.ok(names.has("metier_event_etab_type_idx"));
    assert.ok(names.has("metier_event_eleve_idx"));
  } finally {
    for (const stmt of statements) {
      try {
        await sql.unsafe(stmt);
      } catch {
        /* rétablir après DROP */
      }
    }
    await sql.end({ timeout: 5 });
  }
});
