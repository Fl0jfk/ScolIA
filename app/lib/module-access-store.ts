import "server-only";

import { getJson, putJson } from "@/app/lib/s3-storage";
import {
  defaultModuleAccess,
  parseModuleAccess,
  type ModuleAccessConfig,
} from "@/app/lib/module-access";
import { valkeyDel, valkeyGetJson, valkeySetJson } from "@/app/lib/valkey";
import { VALKEY_TTL, valkeyKeyModuleAccessConfig } from "@/app/lib/valkey-keys";

const CACHE_MS = 30_000;
let cache: { at: number; config: ModuleAccessConfig } | null = null;

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
  if (cache && Date.now() - cache.at < CACHE_MS) return cache.config;
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
}

export async function saveModuleAccess(config: ModuleAccessConfig): Promise<ModuleAccessConfig> {
  const parsed = parseModuleAccess(config);
  await putJson("settings/module-access.json", parsed);
  cache = { at: Date.now(), config: parsed };
  const vk = valkeyKeyModuleAccessConfig(tenantKey());
  void valkeySetJson(vk, parsed, VALKEY_TTL.moduleAccessConfig);
  return parsed;
}
