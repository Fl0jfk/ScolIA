import "server-only";
import { cache } from "react";
import { headers } from "next/headers";
import { TENANT_SLUG_HEADER, type TenantConfig } from "@/app/lib/tenant-types";
import {
  defaultTenantFromEnv,
  isMultiTenantEnabled,
  loadAllTenants,
  normalizeHostname,
  resolveLocalDevTenantFromList,
  resolveTenantByHostname,
  resolveTenantBySlug,
} from "@/app/lib/tenant-registry";
import { isLocalDevHostname } from "@/app/lib/clerk-tenant-keys";
import { tenantOrigin } from "@/app/lib/tenant-auth-urls";
import { isPlatformHostname } from "@/app/lib/platform-hostname";
import { platformTenantFromEnv } from "@/app/lib/platform-tenant";

/** Tenant courant (une requête serveur). */
export const getTenant = cache(async (): Promise<TenantConfig> => {
  const h = await headers();
  const slug = h.get(TENANT_SLUG_HEADER)?.trim();
  if (slug) {
    const bySlug = await resolveTenantBySlug(slug);
    if (bySlug) return bySlug;
  }

  const host = h.get("x-forwarded-host") || h.get("host") || "";
  try {
    return await resolveTenantByHostname(host);
  } catch {
    if (!isMultiTenantEnabled()) {
      return defaultTenantFromEnv();
    }
    const normalized = normalizeHostname(host);
    if (isPlatformHostname(normalized)) {
      return platformTenantFromEnv();
    }
    if (isLocalDevHostname(normalized)) {
      const tenants = await loadAllTenants();
      const devTenant = resolveLocalDevTenantFromList(tenants);
      if (devTenant) return devTenant;
    }
    throw new Error(`Tenant introuvable pour « ${normalized} ».`);
  }
});

export async function getTenantAppUrl(): Promise<string> {
  const tenant = await getTenant();
  const h = await headers();
  const host = h.get("x-forwarded-host") || h.get("host") || "";
  if (host) {
    return tenantOrigin(tenant, host).replace(/\/$/, "");
  }
  return (process.env.NEXT_PUBLIC_APP_URL || "").replace(/\/$/, "");
}

/** URL absolue sur le tenant courant (e-mails, signatures, liens métier). */
export async function tenantAbsolutePath(path: string): Promise<string> {
  const base = await getTenantAppUrl();
  const p = path.startsWith("/") ? path : `/${path}`;
  return `${base}${p}`;
}
