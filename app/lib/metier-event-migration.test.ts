/**
 * Vérifie la migration 0060_metier_event (SQL idempotent).
 * Uniquement TEST_DATABASE_URL + garde local (jamais DATABASE_URL / .env).
 *
 *   TEST_DATABASE_URL=postgresql://scolia@127.0.0.1:5432/scolia_migrate npm run test:metier-event-migration
 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";
import { beginTestDatabase, endTestDatabase } from "@/app/lib/test-database-harness";

function splitMigrationStatements(fileContent: string): string[] {
  return fileContent
    .split(/--> statement-breakpoint\n?/g)
    .map((s) => s.trim())
    .filter(Boolean);
}

test("0060_metier_event — CREATE idempotent (simulation prod sans table)", async (t) => {
  const testDb = beginTestDatabase(t);
  if (!testDb) return;

  const migrationPath = path.join(process.cwd(), "drizzle", "0060_metier_event.sql");
  const statements = splitMigrationStatements(readFileSync(migrationPath, "utf8"));
  assert.ok(statements.length >= 4);

  const postgres = (await import("postgres")).default;
  const sql = postgres(testDb.url, { max: 1 });

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
    await endTestDatabase();
  }
});
