import "server-only";

import { getJson, putJson } from "@/app/lib/s3-storage";
import {
  defaultModuleAccess,
  parseModuleAccess,
  type ModuleAccessConfig,
} from "@/app/lib/module-access";
import { valkeyDel, valkeyGetJson, valkeySetJson } from "@/app/lib/valkey";
import { VALKEY_TTL, valkeyKeyModuleAccessConfig } from "@/app/lib/valkey-keys";

const SOFT_TTL_MS = 120_000;
const HARD_TTL_MS = 15 * 60_000;
let cache: { at: number; config: ModuleAccessConfig } | null = null;
let refreshInFlight: Promise<ModuleAccessConfig> | null = null;

function tenantKey(): string {
  return process.env.DEFAULT_TENANT_SLUG?.trim() || "default";
}

export function getModuleAccessSync(): ModuleAccessConfig {
  return cache?.config ?? defaultModuleAccess();
}

export function invalidateModuleAccessCache(): void {
  cache = null;
  void valkeyDel(valkeyKeyModuleAccessConfig(tenantKey()));
}

export async function loadModuleAccess(): Promise<ModuleAccessConfig> {
  const now = Date.now();
  if (cache && now - cache.at < SOFT_TTL_MS) return cache.config;
  if (cache && now - cache.at < HARD_TTL_MS) {
    void refreshModuleAccessBackground();
    return cache.config;
  }
  return loadModuleAccessFresh();
}

function refreshModuleAccessBackground(): void {
  if (refreshInFlight) return;
  refreshInFlight = loadModuleAccessFresh()
    .catch((error) => {
      console.error("[module-access] refresh background", error);
      return cache?.config ?? defaultModuleAccess();
    })
    .finally(() => {
      refreshInFlight = null;
    });
}

async function loadModuleAccessFresh(): Promise<ModuleAccessConfig> {
  if (refreshInFlight) return refreshInFlight;
  refreshInFlight = (async () => {
    const vk = valkeyKeyModuleAccessConfig(tenantKey());
    const fromValkey = await valkeyGetJson<ModuleAccessConfig>(vk);
    if (fromValkey) {
      cache = { at: Date.now(), config: fromValkey };
      return fromValkey;
    }
    try {
      const raw = await getJson<unknown>("settings/module-access.json");
      const config = raw?.data ? parseModuleAccess(raw.data) : defaultModuleAccess();
      cache = { at: Date.now(), config };
      void valkeySetJson(vk, config, VALKEY_TTL.moduleAccessConfig);
      return config;
    } catch (error) {
      console.error("[module-access] load", error);
      const config = defaultModuleAccess();
      cache = { at: Date.now(), config };
      return config;
    }
  })();
  try {
    return await refreshInFlight;
  } finally {
    refreshInFlight = null;
  }
}

export async function saveModuleAccess(config: ModuleAccessConfig): Promise<ModuleAccessConfig> {
  const parsed = parseModuleAccess(config);
  await putJson("settings/module-access.json", parsed);
  cache = { at: Date.now(), config: parsed };
  const vk = valkeyKeyModuleAccessConfig(tenantKey());
  void valkeySetJson(vk, parsed, VALKEY_TTL.moduleAccessConfig);
  return parsed;
}
