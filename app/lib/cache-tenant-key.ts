import "server-only";

/**
 * Clé tenant pour caches process / Valkey.
 * Préfère `x-tenant-slug` (requête) ; repli `DEFAULT_TENANT_SLUG` / `default` hors HTTP.
 */
export async function resolveCacheTenantSlug(): Promise<string> {
  try {
    const { getTenant } = await import("@/app/lib/tenant-context");
    const tenant = await getTenant();
    const slug = tenant?.slug?.trim().toLowerCase();
    if (slug) return slug;
  } catch {
    /* hors requête / headers absents (scripts, jobs) */
  }
  return process.env.DEFAULT_TENANT_SLUG?.trim().toLowerCase() || "default";
}
