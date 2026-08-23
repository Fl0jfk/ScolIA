import { isLocalDevHostname } from "@/app/lib/local-host-keys";
import { isPlatformTenantSlug } from "@/app/lib/platform-tenant";
import { normalizeHostname } from "@/app/lib/tenant-registry";

const DEFAULT_PLATFORM_HOSTS = ["scolia.fr", "www.scolia.fr"];

function parseHostnameList(raw: string | undefined): string[] {
  if (!raw?.trim()) return [];
  return raw
    .split(",")
    .map((h) => normalizeHostname(h))
    .filter(Boolean);
}

function hostnamesFromAppUrlEnv(): string[] {
  const out: string[] = [];
  for (const raw of [
    process.env.PLATFORM_APP_URL,
    process.env.NEXT_PUBLIC_PLATFORM_APP_URL,
    process.env.NEXT_PUBLIC_APP_URL,
  ]) {
    const value = raw?.trim();
    if (!value) continue;
    try {
      const withScheme = value.startsWith("http") ? value : `https://${value}`;
      const hostname = normalizeHostname(new URL(withScheme).hostname);
      // NEXT_PUBLIC_APP_URL=http://localhost:3000 sert à l'app, pas à déclarer localhost vitrine plateforme.
      if (isLocalDevHostname(hostname)) continue;
      out.push(hostname);
    } catch {
      /* ignore */
    }
  }
  return out;
}

/** Hostnames de la vitrine plateforme (scolia.fr), distincts des tenants établissements. */
export function platformHostnames(): string[] {
  const fromEnv = parseHostnameList(process.env.PLATFORM_HOSTNAMES);
  const devHosts = parseHostnameList(process.env.PLATFORM_DEV_HOSTNAMES);
  const fromAppUrl = hostnamesFromAppUrlEnv();
  const base = fromEnv.length > 0 ? fromEnv : DEFAULT_PLATFORM_HOSTS;
  return [...new Set([...base, ...devHosts, ...fromAppUrl])];
}

export function isPlatformHostname(hostname: string): boolean {
  const host = normalizeHostname(hostname);
  if (!host) return false;
  // En local, vitrine plateforme seulement si PLATFORM_DEV_HOSTNAMES le demande explicitement.
  if (isLocalDevHostname(host)) {
    return parseHostnameList(process.env.PLATFORM_DEV_HOSTNAMES).includes(host);
  }
  return platformHostnames().includes(host);
}
