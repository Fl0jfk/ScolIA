import type { DashboardSignalSourceId } from "@/app/lib/dashboard-signals-parallel";

/** Hash stable pour clés cache (modules accessibles pertinents par source). */
export function dashboardSignalsScopeHash(parts: string[]): string {
  return parts
    .map((p) => p.trim())
    .filter(Boolean)
    .sort()
    .join("|");
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
