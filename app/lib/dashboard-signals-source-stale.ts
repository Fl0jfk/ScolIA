import "server-only";

import type { DashboardSignalSourceId } from "@/app/lib/dashboard-signals-parallel";
import type { SourceStaleAdapter } from "@/app/lib/dashboard-signals-parallel";
import { getDashboardSignalsBundleGeneration } from "@/app/lib/dashboard-signals-bundle-generation-server";
import { valkeyGetJson, valkeySetJson } from "@/app/lib/valkey";
import { VALKEY_TTL } from "@/app/lib/valkey-keys";

const NS = "scola";

const BUNDLE_TTL_MIN_SEC = 120;
const BUNDLE_TTL_MAX_SEC = 180;

export function resolveDashboardSignalsSourceBundleTtlSec(): number {
  const raw = process.env.DASHBOARD_SIGNAL_SOURCE_BUNDLE_TTL_SEC?.trim();
  const n = raw ? Number(raw) : NaN;
  if (Number.isFinite(n) && n >= BUNDLE_TTL_MIN_SEC && n <= BUNDLE_TTL_MAX_SEC) {
    return Math.floor(n);
  }
  return VALKEY_TTL.dashboardSignalsSourceBundle;
}

export function valkeyKeyDashboardSignalSourceBundle(
  etablissementId: string,
  authUserId: string,
  rolesHash: string,
  sourceId: DashboardSignalSourceId,
): string {
  const roles = rolesHash.trim() || "_";
  return `${NS}:dash:sig-bundle:${etablissementId.trim()}:${authUserId.trim()}:${roles}:${sourceId}`;
}

export function buildDashboardSourceStaleAdapters(
  etablissementId: string,
  authUserId: string,
  rolesHash: string,
): Partial<Record<DashboardSignalSourceId, SourceStaleAdapter<unknown>>> {
  const etab = etablissementId.trim() || "default";
  const user = authUserId.trim();
  const roles = rolesHash.trim() || "_";
  if (!user) return {};

  const adapters: Partial<Record<DashboardSignalSourceId, SourceStaleAdapter<unknown>>> = {};
  for (const sourceId of [
    "internat",
    "rh",
    "demandes",
    "absences",
    "documents",
    "stages",
    "others",
  ] as DashboardSignalSourceId[]) {
    const key = valkeyKeyDashboardSignalSourceBundle(etab, user, roles, sourceId);
    adapters[sourceId] = {
      readStale: async () => valkeyGetJson<unknown>(key),
      beginLoad: () => getDashboardSignalsBundleGeneration(etab),
      writeFresh: async (value: unknown, loadGeneration?: number) => {
        if (loadGeneration !== undefined) {
          const current = await getDashboardSignalsBundleGeneration(etab);
          if (current !== loadGeneration) return;
        }
        await valkeySetJson(key, value, resolveDashboardSignalsSourceBundleTtlSec());
      },
    };
  }
  return adapters;
}
