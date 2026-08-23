import "server-only";

import { eq } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { etablissement } from "@/db/schema";
import type { TenantConfig } from "@/app/lib/tenant-types";
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
