import "server-only";

import { getJson, putJson } from "@/app/lib/s3-storage";
import {
  defaultModuleAccess,
  parseModuleAccess,
  type ModuleAccessConfig,
} from "@/app/lib/module-access";

const CACHE_MS = 30_000;
let cache: { at: number; config: ModuleAccessConfig } | null = null;

export function getModuleAccessSync(): ModuleAccessConfig {
  return cache?.config ?? defaultModuleAccess();
}

export function invalidateModuleAccessCache(): void {
  cache = null;
}

export async function loadModuleAccess(): Promise<ModuleAccessConfig> {
  if (cache && Date.now() - cache.at < CACHE_MS) return cache.config;
  try {
    const raw = await getJson<unknown>("settings/module-access.json");
    const config = raw?.data ? parseModuleAccess(raw.data) : defaultModuleAccess();
    cache = { at: Date.now(), config };
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
  return parsed;
}
