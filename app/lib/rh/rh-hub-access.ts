import { hasRole } from "@/app/lib/intranet-role-utils";
import { isAnyDirectionRole } from "@/app/lib/establishment-catalog";

const OGEC_STAFF_ROLES = [
  "administratif",
  "maintenance",
  "surveillant",
  "cpe",
  "comptabilite",
] as const;

function roleFlags(roles: string[]) {
  return {
    isAdmin: hasRole(roles, "admin"),
    isCompta: hasRole(roles, "comptabilite"),
    isAdministratif: hasRole(roles, "administratif"),
    isDirection: isAnyDirectionRole(roles),
    isTeacher: hasRole(roles, "professeur"),
    isOgecStaff: OGEC_STAFF_ROLES.some((r) => hasRole(roles, r)),
  };
}

/** Annuaire, entrées/sorties, onboarding, registre — admin général, compta, directions. */
export function canAccessRhDirectoryViews(roles: string[]) {
  const f = roleFlags(roles);
  return f.isAdmin || f.isCompta || f.isDirection;
}

/** Édition planning (et import PDF) — direction, administratif, compta, admin général. */
export function canEditRhPlanning(roles: string[]) {
  const f = roleFlags(roles);
  return f.isAdmin || f.isCompta || f.isAdministratif || f.isDirection;
}

/** Onglet Demande RH — personnel OGEC uniquement (pas les profs seuls). */
export function canAccessRhStaffRequest(roles: string[]) {
  const f = roleFlags(roles);
  if (f.isTeacher && !f.isDirection && !f.isAdmin) return false;
  return f.isOgecStaff || f.isDirection || f.isAdmin;
}

/** Tableau de bord « Pilotage RH » (stats, mood admin). */
export function canAccessRhPilotageDashboard(roles: string[]) {
  return canAccessRhDirectoryViews(roles);
}
