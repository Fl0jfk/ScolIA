import { loadAppConfig, invalidateAppConfigCache } from "@/app/lib/app-config";

export type TransportProvider = { name: string; email: string };

let cachedProviders: TransportProvider[] | null = null;
let cacheAt = 0;
const CACHE_MS = 45_000;

function invalidateTransportProvidersCache() {
  cachedProviders = null;
  cacheAt = 0;
  invalidateAppConfigCache();
}

export async function getTransportProviders(): Promise<TransportProvider[]> {
  if (cachedProviders && Date.now() - cacheAt < CACHE_MS) return cachedProviders;
  const config = await loadAppConfig();
  cachedProviders = config.travels.transportProviders.map((p) => ({ ...p }));
  cacheAt = Date.now();
  return cachedProviders;
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
