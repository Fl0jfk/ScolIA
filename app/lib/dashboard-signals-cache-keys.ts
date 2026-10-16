import type { DashboardSignalSourceId } from "@/app/lib/dashboard-signals-parallel";

/** Hash stable pour clés cache (modules accessibles pertinents par source). */
export function dashboardSignalsScopeHash(parts: string[]): string {
  return parts
    .map((p) => p.trim())
    .filter(Boolean)
    .sort()
    .join("|");
}

/** Hash stable des rôles session (clés bundle stale par utilisateur). */
export function dashboardSignalsRolesHash(roles: string[]): string {
  return dashboardSignalsScopeHash(roles);
}

export function vsModulesScopeHash(accessibleModuleIds: Set<string>): string {
  const vs = [
    "vs-absences",
    "vs-appels",
    "vs-calendrier",
    "vs-sanctions",
    "vs-carnet",
  ].filter((id) => accessibleModuleIds.has(id));
  return dashboardSignalsScopeHash(vs);
}

export type DashboardSignalsPerfSteps = Record<DashboardSignalSourceId | string, number>;
