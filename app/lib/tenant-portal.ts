import { isLocalDevHostname } from "@/app/lib/local-host-keys";
import { LOCAL_DEV_TENANT_QUERY } from "@/app/lib/local-dev";
import { isPlatformTenantSlug } from "@/app/lib/platform-tenant";
import { resolveTenantCatalogLogo } from "@/app/lib/tenant-catalog-logos";
import { loadAllTenants } from "@/app/lib/tenant-registry";
import type { TenantConfig, TenantPostalAddress } from "@/app/lib/tenant-types";

type PublicTenantCatalogEntry = {
  slug: string;
  kind: string;
  kindLabel: string;
  label: string;
  appUrl: string;
  primaryHostname: string | null;
  postalAddress: TenantPostalAddress | null;
  postalAddressLabel: string;
  logoUrl: string | null;
  signInUrl: string;
};

function kindLabel(kind: string): string {
  return kind === "standalone" ? "École" : "Groupe scolaire";
}

function formatPostalAddress(addr?: TenantPostalAddress | null): string {
  if (!addr) return "";
  const line2 = [addr.zip, addr.city].filter(Boolean).join(" ");
  return [addr.street, line2].filter(Boolean).join(", ");
}

function primaryHostname(tenant: TenantConfig): string | null {
  const host = tenant.hostnames.find((h) => !isLocalDevHostname(h));
  return host ?? tenant.hostnames[0] ?? null;
}

/** URL de connexion pour un tenant (origine du sous-domaine + /auth/sign-in). */
export function tenantSignInUrl(tenant: TenantConfig, portalHost: string): string {
  if (isLocalDevHostname(portalHost)) {
    return `/auth/sign-in?${LOCAL_DEV_TENANT_QUERY}=${encodeURIComponent(tenant.slug)}`;
  }

  const appUrl = tenant.appUrl?.trim().replace(/\/$/, "");
  if (appUrl) {
    try {
      const origin = appUrl.startsWith("http") ? appUrl : `https://${appUrl}`;
      return `${new URL(origin).origin}/auth/sign-in`;
    } catch {
      /* fall through */
    }
  }

  const host = primaryHostname(tenant);
  if (host && !isLocalDevHostname(host)) {
    return `https://${host}/auth/sign-in`;
  }

  return "/auth/sign-in";
}

function tenantToCatalogEntry(
  tenant: TenantConfig,
  portalHost: string,
): PublicTenantCatalogEntry {
  const addressLabel = formatPostalAddress(tenant.postalAddress);
  return {
    slug: tenant.slug,
    kind: tenant.kind,
    kindLabel: kindLabel(tenant.kind),
    label: tenant.label,
    appUrl: tenant.appUrl,
    primaryHostname: primaryHostname(tenant),
    postalAddress: tenant.postalAddress ?? null,
    postalAddressLabel: addressLabel,
    logoUrl: tenant.logoUrl?.trim() || null,
    signInUrl: tenantSignInUrl(tenant, portalHost),
  };
}

/** Libellé pour <select> : nom — type — adresse postale */
function tenantSelectLabel(entry: PublicTenantCatalogEntry): string {
  const address = entry.postalAddressLabel || "Adresse non renseignée";
  return `${entry.label} — ${entry.kindLabel} — ${address}`;
}

/** Établissements visibles sur le portail scola.fr (hors tenant plateforme). */
export async function loadPublicTenantCatalog(portalHost: string): Promise<PublicTenantCatalogEntry[]> {
  const tenants = await loadAllTenants().then((list) =>
    list.filter((t) => !isPlatformTenantSlug(t.slug)),
  );

  const entries = await Promise.all(
    tenants.map(async (tenant) => {
      const entry = tenantToCatalogEntry(tenant, portalHost);
      entry.logoUrl = await resolveTenantCatalogLogo(tenant);
      return entry;
    }),
  );

  return entries.sort((a, b) => a.label.localeCompare(b.label, "fr", { sensitivity: "base" }));
}
