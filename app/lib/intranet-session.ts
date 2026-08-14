import type { User } from "@clerk/backend";
import { auth } from "@clerk/nextjs/server";
import { isClerkDynamicKeyError } from "@/app/lib/clerk-request-error";
import { hasGlobalAdminRole, hasMasterRole, intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { isMultiTenantEnabled } from "@/app/lib/tenant-registry";
import { resolveTenantCurrentUser, resolveTenantSession } from "@/app/lib/tenant-session";

type ResolvedSession = {
  userId: string;
};

export function isPlatformMasterFromPublicMetadata(meta: unknown): boolean {
  return hasMasterRole(intranetRolesFromMetadata(meta));
}

/** Admin établissement (paramètres tenant, relance onboarding) — pas la config plateforme. */
function isTenantAdminFromPublicMetadata(meta: unknown): boolean {
  const m = meta as Record<string, unknown> | undefined;
  if (!m) return false;
  if (m.org_admin === true) return true;
  return hasGlobalAdminRole(intranetRolesFromMetadata(m));
}

/** Accès aux modules orgAdminOnly et APIs admin tenant. */
export function isOrgAdminFromPublicMetadata(meta: unknown): boolean {
  const m = meta as Record<string, unknown> | undefined;
  if (!m) return false;
  if (isPlatformMasterFromPublicMetadata(m)) return true;
  if (m.platform_admin === true) return true;
  return isTenantAdminFromPublicMetadata(m);
}

/** Session : utilisateur Clerk connecté (une instance = une app Clerk + un bucket). */
export async function resolveSession(): Promise<ResolvedSession | null> {
  if (isMultiTenantEnabled()) {
    try {
      return await resolveTenantSession();
    } catch (error) {
      console.error("[resolveSession:tenant]", error);
      return null;
    }
  }

  try {
    const { userId } = await auth();
    if (!userId) return null;
    return { userId };
  } catch (error) {
    if (isClerkDynamicKeyError(error)) throw error;
    console.error("[resolveSession]", error);
    return null;
  }
}

type IntranetClerkUser = User;

/** Profil Clerk courant — en multi-tenant, via la clé secrète du tenant (pas auth() Next). */
export async function safeCurrentUser(): Promise<User | null> {
  if (isMultiTenantEnabled()) {
    try {
      return await resolveTenantCurrentUser();
    } catch (error) {
      console.error("[safeCurrentUser:tenant]", error);
      return null;
    }
  }

  try {
    const { currentUser } = await import("@clerk/nextjs/server");
    return await currentUser();
  } catch (error) {
    if (isClerkDynamicKeyError(error)) throw error;
    console.error("[safeCurrentUser]", error);
    return null;
  }
}

async function isCurrentUserAdmin(): Promise<boolean> {
  const user = await safeCurrentUser();
  return isOrgAdminFromPublicMetadata(user?.publicMetadata);
}
