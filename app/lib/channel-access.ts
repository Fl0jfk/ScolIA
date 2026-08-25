/**
 * Canaux d’accès ScolIA : web établissement vs app (famille / élève / staff-lite).
 * Les comptes sans membership staff n’ont pas le web intranet.
 */

import { isEleveOnlyRoleSet } from "@/app/lib/intranet-role-utils";

const FAMILY_ROLE = new Set(["parent", "eleve"]);

/** Préfixes autorisés pour parent-only / élève-only (app + APIs). */
export const APP_ONLY_ALLOWED_PREFIXES = [
  "/app-mobile",
  "/api/famille",
  "/api/eleve",
  "/api/mobile",
  "/famille", // coque web temporaire en attendant le store
  "/api/auth",
  "/api/account",
  "/api/app/context",
  "/api/tenant/public",
  "/api/tenant/diagnostics",
  "/sign-in",
  "/sign-up",
  "/sign-out",
  "/sso-callback",
  "/auth",
  "/connexion",
] as const;

export function isFamilyOnlyRoleSet(roles: string[]): boolean {
  const visible = roles.filter((r) => r !== "master");
  if (visible.length === 0) return false;
  return visible.every((r) => FAMILY_ROLE.has(r));
}

/** Au moins un rôle personnel (intranet web). */
export function hasStaffCapableRole(roles: string[]): boolean {
  if (roles.includes("admin") || roles.includes("master")) return true;
  return roles.some((r) => !FAMILY_ROLE.has(r));
}

export function isAppOnlyAllowedPath(pathname: string): boolean {
  const normalized = pathname.replace(/\/+$/, "") || "/";
  return APP_ONLY_ALLOWED_PREFIXES.some(
    (p) => normalized === p || normalized.startsWith(`${p}/`),
  );
}

/**
 * Mode canal pour le proxy.
 * - staff : intranet web + éventuellement APIs famille si aussi parent
 * - app_only : uniquement préfixes app (parent / élève sans staff)
 */
export function resolveChannelMode(opts: {
  roles: string[];
  platformAdmin?: boolean;
  orgAdmin?: boolean;
  hasStaffMembership: boolean;
  hasParentOrEleveSignal: boolean;
}): "staff" | "app_only" {
  if (opts.platformAdmin || opts.orgAdmin) return "staff";
  if (opts.hasStaffMembership || hasStaffCapableRole(opts.roles)) return "staff";
  if (
    isFamilyOnlyRoleSet(opts.roles) ||
    isEleveOnlyRoleSet(opts.roles) ||
    opts.hasParentOrEleveSignal
  ) {
    return "app_only";
  }
  // Compte sans rôle ni signal famille : fail-closed app (pas d’intranet).
  return "app_only";
}
