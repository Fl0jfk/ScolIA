import "server-only";

import { getConventionsIndex, getStageConvention } from "@/app/lib/stage-storage";
import type { StageConvention, StageConventionStatus } from "@/app/lib/stage-types";

const DEFAULT_CONCURRENCY = 8;

/**
 * Charge des conventions par id avec un plafond de parallélisme.
 * Un `Promise.all` naïf sur des centaines d’ids saturaient Postgres et
 * ralentissaient tout le site (dashboard, stages, relances groupées…).
 */
export async function loadStageConventionsByIds(
  ids: string[],
  concurrency = DEFAULT_CONCURRENCY,
): Promise<StageConvention[]> {
  const unique = [...new Set(ids.map((id) => id.trim()).filter(Boolean))];
  if (unique.length === 0) return [];

  const out: StageConvention[] = [];
  const limit = Math.max(1, Math.min(concurrency, unique.length));
  let cursor = 0;

  async function worker() {
    while (cursor < unique.length) {
      const idx = cursor;
      cursor += 1;
      const id = unique[idx]!;
      const convention = await getStageConvention(id);
      if (convention) out.push(convention);
    }
  }

  await Promise.all(Array.from({ length: limit }, () => worker()));
  return out;
}

const SKIP_STATUSES: ReadonlySet<StageConventionStatus> = new Set([
  "archived",
  "draft",
  "cancelled",
]);

/** Index filtré puis chargement plafonné — pour le hub stages. */
export async function loadActiveStageConventions(
  concurrency = DEFAULT_CONCURRENCY,
): Promise<StageConvention[]> {
  const index = await getConventionsIndex();
  const ids = index.filter((e) => !SKIP_STATUSES.has(e.status)).map((e) => e.id);
  return loadStageConventionsByIds(ids, concurrency);
}

/** Uniquement les conventions en signatures en cours (signaux dashboard). */
export async function loadSignaturesPendingStageConventions(
  concurrency = DEFAULT_CONCURRENCY,
): Promise<StageConvention[]> {
  const index = await getConventionsIndex();
  const ids = index.filter((e) => e.status === "signatures_pending").map((e) => e.id);
  return loadStageConventionsByIds(ids, concurrency);
}
