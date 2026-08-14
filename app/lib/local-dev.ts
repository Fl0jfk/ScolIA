/** Utilitaires développement local (localhost / 127.0.0.1). */

const LOCAL_HOSTS = new Set(["localhost", "127.0.0.1"]);

export const LOCAL_DEV_TENANT_COOKIE = "scola_dev_tenant";
export const LOCAL_DEV_TENANT_QUERY = "dev_tenant";

function isLocalDevHostname(hostname: string): boolean {
  const raw = (hostname || "").trim().toLowerCase().replace(/:\d+$/, "");
  return LOCAL_HOSTS.has(raw);
}

/** true côté navigateur uniquement */
export function isBrowserLocalDev(): boolean {
  if (typeof window === "undefined") return false;
  return isLocalDevHostname(window.location.hostname);
}

/** Origine http(s) de la requête courante (avec port en dev). */
export function requestOriginFromHostHeader(hostHeader: string): string {
  const raw = (hostHeader || "").trim();
  if (!raw) return "http://localhost:3000";
  const hostname = raw.toLowerCase();
  const proto = isLocalDevHostname(hostname) ? "http" : "https";
  return `${proto}://${raw.replace(/\/$/, "")}`;
}

export function localDevSignInUrl(origin: string, tenantSlug?: string): string {
  const url = new URL("/sign-in", origin);
  if (tenantSlug?.trim()) {
    url.searchParams.set(LOCAL_DEV_TENANT_QUERY, tenantSlug.trim());
  }
  return url.toString();
}

export function localDevDashboardUrl(origin: string): string {
  return new URL("/dashboard", origin).toString();
}
