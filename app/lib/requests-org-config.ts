import { getJson, putJson } from "@/app/lib/s3-storage";
import {
  parseRequestsOrg,
  unwrapRequestsSettingsPayload,
  type RequestsOrgConfig,
  type RequestsRoutingConfig,
} from "@/app/lib/app-config-schemas";
import { saveStaffDirectory } from "@/app/lib/app-config";
import { invalidateRequestRoutesCache } from "@/app/lib/requests-routes-cache";
import { getRequestsRoutingConfig } from "@/app/lib/requests-routing-config";
import {
  defaultRequestsOrg,
  mergeStaffDirectoryFromRoutingAndOrg,
} from "@/app/lib/requests-org-shared";

export {
  collectDelegateEmailsFromOrg,
  defaultRequestsOrg,
  findUnitsForBranch,
  getActiveUnits,
  getChildUnits,
  getDescendantUnitIds,
  isGlobalOversightManager,
  isManagerOfUnit,
  isMetierOversightManager,
  metierOversightBranchIdsForEmail,
  branchMatchesMetierOversight,
  mergeStaffDirectoryFromRoutingAndOrg,
  newRequestServiceUnit,
} from "@/app/lib/requests-org-shared";

const ORG_KEY = "settings/requests-org.json";
const CACHE_MS = 45_000;

let cache: { at: number; config: RequestsOrgConfig } | null = null;

export function invalidateRequestsOrgCache() {
  cache = null;
}

export async function getRequestsOrgConfig(): Promise<RequestsOrgConfig> {
  if (cache && Date.now() - cache.at < CACHE_MS) return cache.config;
  let config: RequestsOrgConfig;
  try {
    const raw = await getJson<{ data?: unknown }>(ORG_KEY);
    const payload = unwrapRequestsSettingsPayload(raw?.data ?? null);
    config = payload != null ? parseRequestsOrg(payload) : defaultRequestsOrg();
  } catch (e) {
    console.error("[requests-org-config] load fallback", e);
    config = defaultRequestsOrg();
  }
  cache = { at: Date.now(), config };
  return config;
}

export async function syncStaffDirectoryFromRequestsConfig(
  routing?: RequestsRoutingConfig,
  org?: RequestsOrgConfig,
): Promise<void> {
  const [r, o] = await Promise.all([
    routing ?? getRequestsRoutingConfig(),
    org ?? getRequestsOrgConfig(),
  ]);
  await saveStaffDirectory(mergeStaffDirectoryFromRoutingAndOrg(r, o));
  invalidateRequestRoutesCache();
}

export async function saveRequestsOrgConfig(config: RequestsOrgConfig): Promise<void> {
  const parsed = parseRequestsOrg(config);
  await putJson(ORG_KEY, { version: 1, updatedAt: new Date().toISOString(), data: parsed });
  invalidateRequestsOrgCache();
  await syncStaffDirectoryFromRequestsConfig(undefined, parsed);
}
