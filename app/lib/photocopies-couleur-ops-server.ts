import "server-only";

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
import { requireAppUser } from "@/app/lib/app-session";

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

/**
 * Contexte session pour la file impressions.
 * Important : `lookup.userId` = Better-Auth id (clés Droits modules),
 * `lookup.businessUserId` = id métier (createdBy / gate.ctx).
 */
export async function resolvePhotocopiesOpsViewer(): Promise<{
  authUserId: string | null;
  businessUserId: string | null;
  email: string;
  roles: string[];
  lookup: ModuleAccessLookup;
  moduleAccess: ModuleAccessConfig | null;
  opsEmails: string[];
  isOps: boolean;
}> {
  const moduleAccess = await loadModuleAccess().catch(() => null);
  const bundle = await loadAppConfig().catch(() => null);
  const legacyEmails = resolvePhotocopiesOpsEmails(bundle?.notifications ?? null);

  const appUser = await requireAppUser();
  if (appUser.ok) {
    const lookup: ModuleAccessLookup = {
      userId: appUser.user.id,
      businessUserId: appUser.user.businessUserId,
    };
    const opsEmails = await resolvePhotocopiesOpsEmailsWithHandlers({
      notifications: bundle?.notifications ?? null,
      moduleAccess,
      legacyEmails,
    });
    const isOps = isPhotocopiesOpsHandlerResolved({
      email: appUser.user.email,
      opsEmails,
      moduleAccess,
      lookup,
      roles: appUser.user.roles,
    });
    return {
      authUserId: appUser.user.id,
      businessUserId: appUser.user.businessUserId,
      email: appUser.user.email,
      roles: appUser.user.roles,
      lookup,
      moduleAccess,
      opsEmails,
      isOps,
    };
  }

  // Repli session compat (sans Better-Auth AppUser complet)
  try {
    const { safeCurrentUser } = await import("@/app/lib/intranet-session");
    const { rolesFromUserLike } = await import("@/app/lib/intranet-roles");
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

  return {
    authUserId: null,
    businessUserId: null,
    email: "",
    roles: [],
    lookup: {},
    moduleAccess,
    opsEmails: legacyEmails,
    isOps: false,
  };
}

/**
 * Destinataires mail « à imprimer » : liste legacy + flag réceptionnaire + rôle Accueil.
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
      // Mail : uniquement flag explicite (Droits modules), pas le défaut rôle Accueil
      // (sinon tous les comptes Accueil recevraient chaque validation).
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
