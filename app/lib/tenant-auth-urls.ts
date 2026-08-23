import { isLocalDevHostname } from "@/app/lib/local-host-keys";
import {
  localDevDashboardUrl,
  localDevSignInUrl,
  LOCAL_DEV_TENANT_QUERY,
  requestOriginFromHostHeader,
} from "@/app/lib/local-dev";
import { isPlatformHostname } from "@/app/lib/platform-hostname";
import { isPlatformTenantSlug } from "@/app/lib/platform-tenant";
import { platformAppOrigin } from "@/app/lib/platform-portal-url";
import { normalizeHostname } from "@/app/lib/tenant-registry";
import type { TenantConfig } from "@/app/lib/tenant-types";

export function tenantOrigin(tenant: TenantConfig, host: string): string {
  const normalized = normalizeHostname(host);
  const rawHost = (host || "").trim();

  if (isLocalDevHostname(normalized)) {
    return requestOriginFromHostHeader(rawHost);
  }

  // Si on est déjà sur un hôte légitime du tenant, on y reste (évite tout saut
  // cross-origin dû à un appUrl mal renseigné).
  if (normalized && tenant.hostnames.some((h) => normalizeHostname(h) === normalized)) {
    return `https://${normalized}`;
  }
  return tenantCanonicalOrigin(tenant);
}

/** Origine canonique du tenant (sous-domaine établissement ou vitrine plateforme). */
export function tenantCanonicalOrigin(tenant: TenantConfig): string {
  if (isPlatformTenantSlug(tenant.slug)) {
    return platformAppOrigin();
  }

  const primaryHost = tenant.hostnames.find((h) => !isLocalDevHostname(h));

  // appUrl n'est de confiance que s'il correspond à un hôte déclaré du tenant ;
  // sinon (donnée erronée) on retombe sur le vrai sous-domaine, sans quoi on
  // redirigerait l'utilisateur hors de son intranet (cross-origin).
  const appUrl = tenant.appUrl?.trim();
  if (appUrl) {
    try {
      const parsed = new URL(appUrl.startsWith("http") ? appUrl : `https://${appUrl}`);
      // normalizeHostname enlève www. — un appUrl "https://www.lpnb…" matcherait
      // hostnames ["lpnb…"] puis renverrait origin AVEC www (DNS/SSL cassés).
      const appHost = normalizeHostname(parsed.hostname);
      const declared = tenant.hostnames.find(
        (h) => !isLocalDevHostname(h) && normalizeHostname(h) === appHost,
      );
      if (declared) {
        return `https://${normalizeHostname(declared)}`;
      }
    } catch {
      /* appUrl invalide → repli sur le sous-domaine */
    }
  }

  if (primaryHost) return `https://${normalizeHostname(primaryHost)}`;
  return platformAppOrigin();
}

export function tenantCanonicalHostname(tenant: TenantConfig): string | null {
  try {
    return normalizeHostname(new URL(tenantCanonicalOrigin(tenant)).hostname);
  } catch {
    return null;
  }
}

/** URL absolue après connexion — sur le domaine courant en local, sinon canonique. */
export function afterSignInUrl(tenant: TenantConfig, host: string): string {
  const normalized = normalizeHostname(host);
  if (isLocalDevHostname(normalized)) {
    const origin = requestOriginFromHostHeader(host);
    if (isPlatformTenantSlug(tenant.slug) || isPlatformHostname(normalized)) {
      return `${origin}/plateforme`;
    }
    return localDevDashboardUrl(origin);
  }
  if (isPlatformTenantSlug(tenant.slug) || isPlatformHostname(normalized)) {
    return `${platformAppOrigin()}/plateforme`;
  }
  return `${tenantCanonicalOrigin(tenant)}/dashboard`;
}

export function signInPageUrl(tenant: TenantConfig, host: string): string {
  const normalized = normalizeHostname(host);
  if (isLocalDevHostname(normalized)) {
    const origin = requestOriginFromHostHeader(host);
    if (isPlatformTenantSlug(tenant.slug) || isPlatformHostname(normalized)) {
      return `${origin}/auth/sign-in`;
    }
    return localDevSignInUrl(origin, tenant.slug);
  }
  if (isPlatformTenantSlug(tenant.slug) || isPlatformHostname(normalized)) {
    return `${platformAppOrigin()}/auth/sign-in`;
  }
  return `${tenantCanonicalOrigin(tenant)}/auth/sign-in`;
}
