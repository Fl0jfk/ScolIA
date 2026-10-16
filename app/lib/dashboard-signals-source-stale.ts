import "server-only";

import type { DashboardSignalSourceId } from "@/app/lib/dashboard-signals-parallel";
import type { SourceStaleAdapter } from "@/app/lib/dashboard-signals-parallel";
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
  sourceId: DashboardSignalSourceId,
): string {
  return `${NS}:dash:sig-bundle:${etablissementId.trim()}:${authUserId.trim()}:${sourceId}`;
}

export function buildDashboardSourceStaleAdapters(
  etablissementId: string,
  authUserId: string,
): Partial<Record<DashboardSignalSourceId, SourceStaleAdapter<unknown>>> {
  const etab = etablissementId.trim() || "default";
  const user = authUserId.trim();
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
    const key = valkeyKeyDashboardSignalSourceBundle(etab, user, sourceId);
    adapters[sourceId] = {
      readStale: async () => valkeyGetJson<unknown>(key),
      writeFresh: async (value: unknown) => {
        await valkeySetJson(key, value, resolveDashboardSignalsSourceBundleTtlSec());
      },
    };
  }
  return adapters;
}
