import "server-only";

import { eq } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { etablissement } from "@/db/schema";
import type { TenantConfig } from "@/app/lib/tenant-types";
import { isPlatformTenantSlug } from "@/app/lib/platform-tenant";
import { resolveTenantBySlug } from "@/app/lib/tenant-registry";

export async function ensureEtablissementFromTenant(
  tenant: TenantConfig,
): Promise<string> {
  if (!isDatabaseConfigured()) {
    throw new Error("DATABASE_URL requise pour Better-Auth.");
  }
  const db = getDb();
  const [existing] = await db
    .select({ id: etablissement.id })
    .from(etablissement)
    .where(eq(etablissement.slug, tenant.slug))
    .limit(1);
  if (existing) return existing.id;

  const [created] = await db
    .insert(etablissement)
    .values({
      slug: tenant.slug,
      name: tenant.label?.trim() || tenant.slug,
      dataBucket: tenant.dataBucket,
    })
    .returning({ id: etablissement.id });

  return created.id;
}

export async function resolveEtablissementIdBySlug(slug: string): Promise<string | null> {
  if (!isDatabaseConfigured()) return null;
  const db = getDb();
  const [row] = await db
    .select({ id: etablissement.id })
    .from(etablissement)
    .where(eq(etablissement.slug, slug))
    .limit(1);
  return row?.id ?? null;
}

export async function ensureEtablissementFromSlug(slug: string): Promise<string> {
  const tenant = await resolveTenantBySlug(slug);
  if (!tenant) throw new Error(`Tenant inconnu : ${slug}`);
  return ensureEtablissementFromTenant(tenant);
}

/**
 * Un utilisateur d’établissement ne peut accéder qu’à son tenant.
 * Master plateforme (`platformAdmin`) peut traverser les tenants.
 * Sur le hostname plateforme (scolia.fr), pas de cloisonnement métier.
 */
export async function assertUserBelongsToTenant(opts: {
  userEtablissementId: string | null | undefined;
  platformAdmin: boolean;
  tenant: TenantConfig;
}): Promise<{ ok: true } | { ok: false; code: "TENANT_FORBIDDEN"; message: string }> {
  if (opts.platformAdmin) return { ok: true };
  if (isPlatformTenantSlug(opts.tenant.slug)) return { ok: true };

  const tenantEtablissementId = await ensureEtablissementFromTenant(opts.tenant);
  const userEtab = opts.userEtablissementId?.trim() || "";
  if (!userEtab || userEtab !== tenantEtablissementId) {
    return {
      ok: false,
      code: "TENANT_FORBIDDEN",
      message:
        "Ce compte n’appartient pas à cet établissement. Utilisez le sous-domaine de votre intranet.",
    };
  }
  return { ok: true };
}
