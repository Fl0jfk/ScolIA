import "server-only";

import { getJson, putJson } from "@/app/lib/s3-storage";
import {
  defaultModuleAccess,
  parseModuleAccess,
  type ModuleAccessConfig,
} from "@/app/lib/module-access";
import { resolveCacheTenantSlug } from "@/app/lib/cache-tenant-key";
import { valkeyDel, valkeyGetJson, valkeySetJson } from "@/app/lib/valkey";
import { VALKEY_TTL, valkeyKeyModuleAccessConfig } from "@/app/lib/valkey-keys";

const SOFT_TTL_MS = 120_000;
const HARD_TTL_MS = 15 * 60_000;

type ModuleAccessCacheEntry = { at: number; config: ModuleAccessConfig };

const caches = new Map<string, ModuleAccessCacheEntry>();
const refreshInFlightBySlug = new Map<string, Promise<ModuleAccessConfig>>();

export function getModuleAccessSync(): ModuleAccessConfig {
  // Synchrone : dernier slug vu, sinon défaut (évite de croiser deux tenants).
  if (caches.size === 1) {
    const only = caches.values().next().value;
    if (only) return only.config;
  }
  return defaultModuleAccess();
}

export function invalidateModuleAccessCache(): void {
  caches.clear();
  refreshInFlightBySlug.clear();
  void resolveCacheTenantSlug()
    .then((slug) => valkeyDel(valkeyKeyModuleAccessConfig(slug)))
    .catch(() => undefined);
}

export async function loadModuleAccess(): Promise<ModuleAccessConfig> {
  const slug = await resolveCacheTenantSlug();
  const now = Date.now();
  const hit = caches.get(slug);
  if (hit && now - hit.at < SOFT_TTL_MS) return hit.config;
  if (hit && now - hit.at < HARD_TTL_MS) {
    void refreshModuleAccessBackground(slug);
    return hit.config;
  }
  return loadModuleAccessFresh(slug);
}

function refreshModuleAccessBackground(slug: string): void {
  if (refreshInFlightBySlug.has(slug)) return;
  const p = loadModuleAccessFresh(slug)
    .catch((error) => {
      console.error("[module-access] refresh background", error);
      return caches.get(slug)?.config ?? defaultModuleAccess();
    })
    .finally(() => {
      refreshInFlightBySlug.delete(slug);
    });
  refreshInFlightBySlug.set(slug, p);
}

async function loadModuleAccessFresh(slug: string): Promise<ModuleAccessConfig> {
  const inflight = refreshInFlightBySlug.get(slug);
  if (inflight) return inflight;

  const promise = (async () => {
    const vk = valkeyKeyModuleAccessConfig(slug);
    const fromValkey = await valkeyGetJson<ModuleAccessConfig>(vk);
    if (fromValkey) {
      caches.set(slug, { at: Date.now(), config: fromValkey });
      return fromValkey;
    }
    try {
      const raw = await getJson<unknown>("settings/module-access.json");
      const config = raw?.data ? parseModuleAccess(raw.data) : defaultModuleAccess();
      caches.set(slug, { at: Date.now(), config });
      void valkeySetJson(vk, config, VALKEY_TTL.moduleAccessConfig);
      return config;
    } catch (error) {
      console.error("[module-access] load", error);
      const config = defaultModuleAccess();
      caches.set(slug, { at: Date.now(), config });
      return config;
    }
  })();

  refreshInFlightBySlug.set(slug, promise);
  try {
    return await promise;
  } finally {
    refreshInFlightBySlug.delete(slug);
  }
}

export async function saveModuleAccess(config: ModuleAccessConfig): Promise<ModuleAccessConfig> {
  const parsed = parseModuleAccess(config);
  await putJson("settings/module-access.json", parsed);
  const slug = await resolveCacheTenantSlug();
  caches.set(slug, { at: Date.now(), config: parsed });
  const vk = valkeyKeyModuleAccessConfig(slug);
  void valkeySetJson(vk, parsed, VALKEY_TTL.moduleAccessConfig);
  return parsed;
}
