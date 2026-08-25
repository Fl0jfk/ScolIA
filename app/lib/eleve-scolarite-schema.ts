import "server-only";

import { sql } from "drizzle-orm";
import { getDb } from "@/db/index";

let ensured = false;

/** Migration légère : colonne grille_repas absente sur certains tenants. */
export async function ensureEleveScolariteGrilleRepasColumn(): Promise<void> {
  if (ensured) return;
  const db = getDb();
  await db.execute(
    sql.raw(`ALTER TABLE "eleve_scolarite" ADD COLUMN IF NOT EXISTS "grille_repas" jsonb`),
  );
  ensured = true;
}
