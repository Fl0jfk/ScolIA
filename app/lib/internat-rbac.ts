import type { Establishment } from "@/app/lib/app-config-schemas";
import { isAnyDirectionRole, internatEligibleEstablishments } from "@/app/lib/establishment-catalog";
import { inferEstablishmentKind } from "@/app/lib/establishment-visual";
import { hasRole } from "@/app/lib/absences-types";
import { hasGlobalAdminRole, intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import type { InternatRollCallRecipients, InternatStudent } from "@/app/lib/internat-types";

export type InternatRollCallViewerScope = "all" | "college" | "lycee";

export function canAccessInternatModule(roles: string[]) {
  if (hasGlobalAdminRole(roles)) return true;
  return (
    hasRole(roles, "surveillant") ||
    hasRole(roles, "internat") ||
    hasRole(roles, "cpe") ||
    hasRole(roles, "administratif") ||
    isAnyDirectionRole(roles)
  );
}

/**
 * Signal dashboard / rappel opérationnel « appel du soir » :
 * uniquement le rôle `internat` (surveillants et CPE gardent l’accès module sans spam quotidien).
 * Affiché uniquement les soirs d’hébergement (lundi → jeudi).
 */
export function canSeeInternatRollCallSignal(roles: string[]) {
  return hasRole(roles, "internat");
}

export function canAccessInternatFromMetadata(meta: unknown) {
  if (isOrgAdminMetadata(meta)) return true;
  return canAccessInternatModule(intranetRolesFromMetadata(meta));
}

export function canManageInternatConfig(roles: string[]) {
  if (hasGlobalAdminRole(roles)) return true;
  return (
    hasRole(roles, "surveillant") ||
    hasRole(roles, "cpe") ||
    hasRole(roles, "administratif") ||
    isAnyDirectionRole(roles)
  );
}

export function isOrgAdminMetadata(meta: unknown) {
  const m = meta as Record<string, unknown> | undefined;
  if (!m) return false;
  if (m.org_admin === true) return true;
  if (m.platform_admin === true) return true;
  return hasGlobalAdminRole(intranetRolesFromMetadata(m));
}

export function rolesFromMetadata(meta: unknown): string[] {
  return intranetRolesFromMetadata(meta);
}

function normEmail(raw: string | null | undefined): string {
  return String(raw || "")
    .trim()
    .toLowerCase();
}

/**
 * Périmètre de consultation de l’appel pour direction / CPE.
 * L’équipe terrain (internat, surveillant, admin) voit toujours tout.
 */
export function resolveInternatRollCallViewerScope(params: {
  roles: string[];
  email?: string | null;
  isOrgAdmin?: boolean;
  recipients?: InternatRollCallRecipients | null;
  establishments?: Establishment[];
}): InternatRollCallViewerScope {
  if (params.isOrgAdmin || hasGlobalAdminRole(params.roles)) return "all";
  if (hasRole(params.roles, "internat") || hasRole(params.roles, "surveillant")) return "all";
  if (hasRole(params.roles, "administratif") && !isAnyDirectionRole(params.roles) && !hasRole(params.roles, "cpe")) {
    return "all";
  }

  const hasCollegeDir = hasRole(params.roles, "direction_college");
  const hasLyceeDir = hasRole(params.roles, "direction_lycee");
  if (hasCollegeDir && hasLyceeDir) return "all";
  if (hasCollegeDir && !hasLyceeDir) return "college";
  if (hasLyceeDir && !hasCollegeDir) return "lycee";

  const email = normEmail(params.email);
  if (email) {
    const r = params.recipients;
    const collegeEmails = new Set(
      [r?.directionCollege, r?.cpeCollege]
        .map(normEmail)
        .filter(Boolean),
    );
    const lyceeEmails = new Set(
      [r?.directionLycee, r?.cpeLycee, r?.appelContact]
        .map(normEmail)
        .filter(Boolean),
    );

    for (const est of internatEligibleEstablishments(params.establishments || [])) {
      const director = normEmail(est.directorEmail);
      if (!director) continue;
      const kind = inferEstablishmentKind(est);
      if (kind === "college") collegeEmails.add(director);
      if (kind === "lycee") lyceeEmails.add(director);
    }

    const inCollege = collegeEmails.has(email);
    const inLycee = lyceeEmails.has(email);
    if (inCollege && !inLycee) return "college";
    if (inLycee && !inCollege) return "lycee";
  }

  return "all";
}

export function filterInternatStudentsByViewerScope(
  students: InternatStudent[],
  scope: InternatRollCallViewerScope,
): InternatStudent[] {
  if (scope === "all") return students;
  return students.filter((s) => inferEstablishmentKind({ label: s.etablissement }) === scope);
}

export function internatRollCallScopeLabel(scope: InternatRollCallViewerScope): string {
  if (scope === "college") return "Collège";
  if (scope === "lycee") return "Lycée";
  return "Tous les établissements";
}
