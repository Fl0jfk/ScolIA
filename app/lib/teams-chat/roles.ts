import { isEleveOnlyRoleSet } from "@/app/lib/intranet-role-utils";

/** Personnel uniquement (pas parents / élèves). */
export function canUseTeamsChatOverlay(roles: string[]): boolean {
  if (isEleveOnlyRoleSet(roles)) return false;
  const visible = roles.filter((r) => r !== "master");
  if (visible.length === 0) return roles.includes("master");
  return visible.some((r) => r !== "parent" && r !== "eleve");
}
