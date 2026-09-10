import "server-only";

import { eq } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { etablissement } from "@/db/schema";
import type { TenantConfig } from "@/app/lib/tenant-types";
import { isPlatformTenantSlug } from "@/app/lib/platform-tenant";
import { resolveTenantBySlug } from "@/app/lib/tenant-registry";
import { userHasActiveMembership } from "@/app/lib/user-membership";

/** Cache process : slug → id (évite 1 SELECT Postgres par navigation proxy). */
const etabIdBySlug = new Map<string, { id: string; expiresAt: number }>();
const ETAB_CACHE_TTL_MS = 10 * 60 * 1000;

export async function ensureEtablissementFromTenant(
  tenant: TenantConfig,
): Promise<string> {
  if (!isDatabaseConfigured()) {
    throw new Error("DATABASE_URL requise pour Better-Auth.");
  }
  const cached = etabIdBySlug.get(tenant.slug);
  if (cached && cached.expiresAt > Date.now()) return cached.id;

  const lookup = async (): Promise<string> => {
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
  };

  let lastError: unknown;
  for (let attempt = 0; attempt < 3; attempt += 1) {
    try {
      const id = await lookup();
      etabIdBySlug.set(tenant.slug, {
        id,
        expiresAt: Date.now() + ETAB_CACHE_TTL_MS,
      });
      return id;
    } catch (error) {
      lastError = error;
      const msg = error instanceof Error ? error.message : String(error);
      // Pool saturé / connexion coupée : courte pause puis retry.
      if (!/53300|too many clients|CONNECT_TIMEOUT|connection|ECONNRESET/i.test(msg)) {
        throw error;
      }
      await new Promise((r) => setTimeout(r, 120 * (attempt + 1)));
    }
  }
  throw lastError instanceof Error
    ? lastError
    : new Error("Impossible de résoudre l’établissement.");
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
 * Un utilisateur ne peut accéder qu’aux établissements où il a un membership actif.
 * Fallback legacy : `user.etablissement_id` (auto-guérit en créant le membership).
 * Master plateforme (`platformAdmin`) peut traverser les tenants.
 * Sur le hostname plateforme (scolia.fr), pas de cloisonnement métier.
 */
export async function assertUserBelongsToTenant(opts: {
  userId: string;
  userEtablissementId: string | null | undefined;
  platformAdmin: boolean;
  tenant: TenantConfig;
}): Promise<{ ok: true } | { ok: false; code: "TENANT_FORBIDDEN"; message: string }> {
  if (opts.platformAdmin) return { ok: true };
  if (isPlatformTenantSlug(opts.tenant.slug)) return { ok: true };

  const tenantEtablissementId = await ensureEtablissementFromTenant(opts.tenant);
  const userId = opts.userId.trim();
  const userEtab = opts.userEtablissementId?.trim() || "";

  // Chemin chaud : même établissement maison → pas de SELECT membership.
  if (userId && userEtab && userEtab === tenantEtablissementId) {
    return { ok: true };
  }

  if (userId && (await userHasActiveMembership(userId, tenantEtablissementId))) {
    return { ok: true };
  }

  return {
    ok: false,
    code: "TENANT_FORBIDDEN",
    message:
      "Ce compte n’appartient pas à cet établissement. Connectez-vous depuis scolia.fr avec votre e-mail.",
  };
}
