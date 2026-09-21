import { loadAppConfig, invalidateAppConfigCache } from "@/app/lib/app-config";

export type TransportProvider = { name: string; email: string };

const providersCacheByTenant = new Map<string, { at: number; providers: TransportProvider[] }>();
const CACHE_MS = 45_000;

function invalidateTransportProvidersCache() {
  providersCacheByTenant.clear();
  invalidateAppConfigCache();
}

export async function getTransportProviders(): Promise<TransportProvider[]> {
  const { resolveCacheTenantSlug } = await import("@/app/lib/cache-tenant-key");
  const slug = await resolveCacheTenantSlug();
  const hit = providersCacheByTenant.get(slug);
  if (hit && Date.now() - hit.at < CACHE_MS) return hit.providers;
  const config = await loadAppConfig();
  const providers = config.travels.transportProviders.map((p) => ({ ...p }));
  providersCacheByTenant.set(slug, { at: Date.now(), providers });
  return providers;
}

const norm = (e: string) => e.trim().toLowerCase();

export async function providerNameFromEmail(fromEmail: string): Promise<string | null> {
  const providers = await getTransportProviders();
  const n = norm(fromEmail);
  const p = providers.find((t) => norm(t.email) === n);
  return p?.name ?? null;
}

function providerNameFromEmailSync(fromEmail: string, providers: TransportProvider[]): string | null {
  const n = norm(fromEmail);
  const p = providers.find((t) => norm(t.email) === n);
  return p?.name ?? null;
}
