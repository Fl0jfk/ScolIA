import "server-only";

import {
  bumpDashboardSignalsBundleGenerationMemory,
  getDashboardSignalsBundleGenerationMemory,
} from "@/app/lib/dashboard-signals-bundle-generation";
import { isValkeyConfigured, valkeyGet, valkeyIncr } from "@/app/lib/valkey";

const NS = "scola";

export function valkeyKeyDashboardSignalsBundleGeneration(etablissementId: string): string {
  return `${NS}:dash:sig-bundle-gen:${etablissementId.trim() || "default"}`;
}

export async function getDashboardSignalsBundleGeneration(
  etablissementId: string,
): Promise<number> {
  const mem = getDashboardSignalsBundleGenerationMemory(etablissementId);
  if (!isValkeyConfigured()) return mem;
  const raw = await valkeyGet(valkeyKeyDashboardSignalsBundleGeneration(etablissementId));
  const remote = raw ? Number(raw) : 0;
  if (!Number.isFinite(remote) || remote < 0) return mem;
  return Math.max(mem, Math.floor(remote));
}

export async function bumpDashboardSignalsBundleGeneration(
  etablissementId: string,
): Promise<number> {
  const memNext = bumpDashboardSignalsBundleGenerationMemory(etablissementId);
  if (!isValkeyConfigured()) return memNext;
  const remote = await valkeyIncr(
    valkeyKeyDashboardSignalsBundleGeneration(etablissementId),
    7 * 24 * 60 * 60,
  );
  if (remote === null || !Number.isFinite(remote)) return memNext;
  return Math.max(memNext, Math.floor(remote));
}
