import "server-only";

import { and, eq, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve } from "@/db/schema";

/**
 * Résout `eleve_key` (INE) → `eleve.id` pour un établissement.
 * Matching strict INE uniquement — pas de flou nom/prénom (revue des orphelins).
 */
export async function resolveEleveIdsByIneKeys(opts: {
  etablissementId: string;
  keys: string[];
}): Promise<Map<string, string>> {
  const normalized = [
    ...new Set(
      opts.keys
        .map((k) => k.trim().toUpperCase())
        .filter((k) => k.length > 0),
    ),
  ];
  const out = new Map<string, string>();
  if (normalized.length === 0) return out;

  const db = getDb();
  const rows = await db
    .select({ id: eleve.id, ine: eleve.ine })
    .from(eleve)
    .where(
      and(
        eq(eleve.etablissementId, opts.etablissementId),
        sql`upper(btrim(COALESCE(${eleve.ine}, ''))) IN (${sql.join(
          normalized.map((k) => sql`${k}`),
          sql`, `,
        )})`,
      ),
    );

  const buckets = new Map<string, string[]>();
  for (const row of rows) {
    const key = row.ine?.trim().toUpperCase() ?? "";
    if (!key) continue;
    const list = buckets.get(key) ?? [];
    list.push(row.id);
    buckets.set(key, list);
  }

  for (const [key, ids] of buckets) {
    const unique = [...new Set(ids)];
    // Plusieurs fiches même INE : ne pas inventer — laisser non résolu.
    if (unique.length === 1) out.set(key, unique[0]!);
  }
  return out;
}

export function normalizeParticipantIneKey(raw: string | null | undefined): string {
  return String(raw ?? "")
    .trim()
    .toUpperCase();
}
