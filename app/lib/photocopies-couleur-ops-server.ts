import "server-only";

import type { AppUser } from "@/app/lib/app-session";
import { requireAppUser, requireViewUser } from "@/app/lib/app-session";
import type { ModuleAccessConfig, ModuleAccessLookup } from "@/app/lib/module-access";
import {
  findUserOverride,
  listUserIdsWithModuleFlag,
  userHasPhotocopiesOpsFlag,
} from "@/app/lib/module-access";
import {
  isPhotocopiesOpsHandler,
  resolvePhotocopiesOpsEmails,
} from "@/app/lib/photocopies-couleur-ops";
import type { NotificationsConfig } from "@/app/lib/app-config-schemas";
import { loadModuleAccess } from "@/app/lib/module-access-store";
import { loadAppConfig } from "@/app/lib/app-config";
import { ensureEtablissementFromTenant } from "@/app/lib/etablissement-db";
import { getTenant } from "@/app/lib/tenant-context";
import { listMembersFromDb } from "@/app/lib/members-db";
import { isDatabaseConfigured } from "@/db/index";

export type PhotocopiesOpsContext = {
  authUserId: string | null;
  businessUserId: string | null;
  email: string;
  roles: string[];
  lookup: ModuleAccessLookup;
  moduleAccess: ModuleAccessConfig | null;
  opsEmails: string[];
  isOps: boolean;
};

/** E-mails legacy OU flag Droits modules OU rôle Accueil (réceptionnaire par défaut). */
export function isPhotocopiesOpsHandlerResolved(opts: {
  email?: string | null;
  opsEmails?: string[];
  notifications?: Pick<NotificationsConfig, "photocopiesOps" | "photocopiesOpsEmails"> | null;
  moduleAccess?: ModuleAccessConfig | null;
  lookup?: ModuleAccessLookup | null;
  roles?: string[] | null;
}): boolean {
  const emails =
    opts.opsEmails ?? resolvePhotocopiesOpsEmails(opts.notifications ?? null);
  if (isPhotocopiesOpsHandler(opts.email, emails)) return true;
  return userHasPhotocopiesOpsFlag(opts.moduleAccess, opts.lookup, opts.roles);
}

async function buildOpsContext(user: AppUser): Promise<PhotocopiesOpsContext> {
  const moduleAccess = await loadModuleAccess().catch(() => null);
  const bundle = await loadAppConfig().catch(() => null);
  const legacyEmails = resolvePhotocopiesOpsEmails(bundle?.notifications ?? null);
  const lookup: ModuleAccessLookup = {
    userId: user.id,
    businessUserId: user.businessUserId,
  };
  const opsEmails = await resolvePhotocopiesOpsEmailsWithHandlers({
    notifications: bundle?.notifications ?? null,
    moduleAccess,
    legacyEmails,
  });
  const isOps = isPhotocopiesOpsHandlerResolved({
    email: user.email,
    opsEmails,
    moduleAccess,
    lookup,
    roles: user.roles,
  });
  return {
    authUserId: user.id,
    businessUserId: user.businessUserId,
    email: user.email,
    roles: user.roles,
    lookup,
    moduleAccess,
    opsEmails,
    isOps,
  };
}

/**
 * Contexte « vu » (cible de supervision si active, sinon acteur).
 * À utiliser pour GET / listes « mes demandes » / PDF.
 */
export async function resolvePhotocopiesOpsViewer(): Promise<PhotocopiesOpsContext> {
  const viewUser = await requireViewUser();
  if (viewUser.ok) return buildOpsContext(viewUser.user);

  // Repli session compat
  try {
    const { safeCurrentUser } = await import("@/app/lib/intranet-session");
    const { rolesFromUserLike } = await import("@/app/lib/intranet-roles");
    const moduleAccess = await loadModuleAccess().catch(() => null);
    const bundle = await loadAppConfig().catch(() => null);
    const legacyEmails = resolvePhotocopiesOpsEmails(bundle?.notifications ?? null);
    const user = await safeCurrentUser();
    if (user) {
      const roles = rolesFromUserLike(user);
      const email = user.primaryEmailAddress?.emailAddress?.trim() || "";
      const lookup: ModuleAccessLookup = {
        userId: null,
        businessUserId: user.id,
      };
      const opsEmails = await resolvePhotocopiesOpsEmailsWithHandlers({
        notifications: bundle?.notifications ?? null,
        moduleAccess,
        legacyEmails,
      });
      const isOps = isPhotocopiesOpsHandlerResolved({
        email,
        opsEmails,
        moduleAccess,
        lookup,
        roles,
      });
      return {
        authUserId: null,
        businessUserId: user.id,
        email,
        roles,
        lookup,
        moduleAccess,
        opsEmails,
        isOps,
      };
    }
  } catch {
    /* ignore */
  }

  const moduleAccess = await loadModuleAccess().catch(() => null);
  const bundle = await loadAppConfig().catch(() => null);
  return {
    authUserId: null,
    businessUserId: null,
    email: "",
    roles: [],
    lookup: {},
    moduleAccess,
    opsEmails: resolvePhotocopiesOpsEmails(bundle?.notifications ?? null),
    isOps: false,
  };
}

/**
 * Acteur réel authentifié (ignore la supervision).
 * À utiliser pour marquer « imprimée / prête » et les décisions direction.
 */
export async function resolvePhotocopiesOpsActor(): Promise<PhotocopiesOpsContext> {
  const actor = await requireAppUser();
  if (actor.ok) return buildOpsContext(actor.user);
  // Même repli que le viewer si pas d’AppUser (dev / session dégradée)
  return resolvePhotocopiesOpsViewer();
}

/**
 * Destinataires mail « à imprimer » : liste legacy + flag réceptionnaire explicite.
 */
export async function resolvePhotocopiesOpsEmailsWithHandlers(opts?: {
  notifications?: Pick<NotificationsConfig, "photocopiesOps" | "photocopiesOpsEmails"> | null;
  moduleAccess?: ModuleAccessConfig | null;
  legacyEmails?: string[];
}): Promise<string[]> {
  const out: string[] = [];
  const seen = new Set<string>();
  const push = (raw: string | undefined | null) => {
    const e = String(raw || "").trim();
    if (!e || !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(e)) return;
    const k = e.toLowerCase();
    if (seen.has(k)) return;
    seen.add(k);
    out.push(e);
  };

  const legacy =
    opts?.legacyEmails ?? resolvePhotocopiesOpsEmails(opts?.notifications ?? null);
  for (const e of legacy) push(e);

  if (!isDatabaseConfigured()) return out;

  try {
    const tenant = await getTenant();
    const etablissementId = await ensureEtablissementFromTenant(tenant);
    if (!etablissementId) return out;
    const members = await listMembersFromDb(etablissementId);
    const moduleAccess = opts?.moduleAccess ?? (await loadModuleAccess().catch(() => null));
    const flaggedIds = new Set(listUserIdsWithModuleFlag(moduleAccess, "photocopiesOps"));

    for (const m of members) {
      const lookup: ModuleAccessLookup = {
        userId: m.userId,
        businessUserId: m.externalUserId,
      };
      const byUserFlag =
        (m.userId ? flaggedIds.has(m.userId) : false) ||
        (m.externalUserId ? flaggedIds.has(m.externalUserId) : false) ||
        findUserOverride(moduleAccess, lookup)?.photocopiesOps === true;
      const byRoleFlag = m.roles.some((r) => moduleAccess?.byRole?.[r]?.photocopiesOps === true);
      if (byUserFlag || byRoleFlag) push(m.email);
    }
  } catch (e) {
    console.error("[photocopies-ops] resolve emails handlers:", e);
  }

  return out;
}
