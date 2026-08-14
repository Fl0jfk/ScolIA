import { getRequestsRoutingConfig } from "@/app/lib/requests-routing-config";
import { BRANCH_TO_SERVICE } from "@/app/lib/requests-routing-defaults";
import { normalizeRequestEmail } from "@/app/lib/requests-board";
import {
  getStaffExecutorsForBranchFromRows,
  getStaffLeadersForBranchFromRows,
  loadStaffDirectoryRows,
} from "@/app/lib/staff-directory";
import type { RequestRouteDef } from "@/app/lib/requests-types";

const CACHE_MS = 45_000;
let routesCache: { at: number; map: Map<string, RequestRouteDef>; routes: RequestRouteDef[] } | null = null;

function invalidateRequestRoutesCache() {
  routesCache = null;
}

export async function ensureRequestRoutes(): Promise<{
  map: Map<string, RequestRouteDef>;
  routes: RequestRouteDef[];
}> {
  if (routesCache && Date.now() - routesCache.at < CACHE_MS) return routesCache;

  const [routing, staffRows] = await Promise.all([getRequestsRoutingConfig(), loadStaffDirectoryRows()]);
  const serviceById = new Map(routing.services.map((s) => [s.id, s]));
  const taskService = new Map<string, string>();
  for (const a of routing.assignments) {
    if (!taskService.has(a.taskId)) taskService.set(a.taskId, a.serviceId);
  }

  const routes: RequestRouteDef[] = routing.tasks.map((task) => {
    const id = task.id;
    const leaders = getStaffLeadersForBranchFromRows(staffRows, id);
    const execs = getStaffExecutorsForBranchFromRows(staffRows, id);
    const pool = [...new Set([...leaders, ...execs].map((e) => normalizeRequestEmail(String(e))).filter(Boolean))];
    const primary = pool[0] || leaders[0] || "";
    const serviceId = taskService.get(id) || BRANCH_TO_SERVICE[id] || "administratif";
    const category = serviceById.get(serviceId)?.category || "Général";
    return {
      id,
      category,
      roleLabel: task.label,
      leaderEmails: () => [...leaders],
      executorEmails: () => [...execs],
      primaryEmail: () => primary,
      ccEmails: () => [],
      poolEmails: () => (pool.length > 0 ? pool : primary ? [primary] : []),
      promptLine: task.hint,
      keywords: [...task.keywords],
    };
  });

  routesCache = {
    at: Date.now(),
    routes,
    map: new Map(routes.map((r) => [r.id, r])),
  };
  return routesCache;
}

export async function getRouteById(routeId: string): Promise<RequestRouteDef | null> {
  const { map } = await ensureRequestRoutes();
  return map.get(routeId) ?? null;
}
