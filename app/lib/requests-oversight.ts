import "server-only";

import type { RequestsOrgConfig } from "@/app/lib/app-config-schemas";
import { normalizeRequestBranchId, normalizeRequestEmail } from "@/app/lib/requests-board";
import {
  branchMatchesMetierOversight,
  isGlobalOversightManager,
} from "@/app/lib/requests-org-shared";

export type ViewerOversightContext = {
  globalOversight: boolean;
  metierBranchIds: string[];
};

export function buildViewerOversightContext(
  org: RequestsOrgConfig,
  email: string,
): ViewerOversightContext {
  const u = normalizeRequestEmail(email);
  if (!u) return { globalOversight: false, metierBranchIds: [] };
  const globalOversight = isGlobalOversightManager(org, u);
  const metierBranchIds = new Set<string>();
  for (const unit of org.units.filter((x: RequestsOrgConfig["units"][number]) => x.active)) {
    if (!(org.metierOversightUnitIds ?? []).includes(unit.id)) continue;
    if (!unit.managerEmails.map(normalizeRequestEmail).includes(u)) continue;
    for (const taskId of unit.taskIds) metierBranchIds.add(taskId);
  }
  return { globalOversight, metierBranchIds: [...metierBranchIds] };
}

type MinimalAssigned = {
  routeId?: string;
  unit: string;
};

export function hasOversightOnBranch(
  oversight: ViewerOversightContext,
  org: RequestsOrgConfig,
  userEmail: string,
  assigned: MinimalAssigned,
): boolean {
  if (oversight.globalOversight) return true;
  const branch = normalizeRequestBranchId(assigned.routeId, assigned.unit);
  if (oversight.metierBranchIds.includes(branch)) return true;
  return branchMatchesMetierOversight(org, userEmail, branch);
}

export function actsAsLeaderWithOversight(
  oversight: ViewerOversightContext,
  org: RequestsOrgConfig,
  userEmail: string,
  assigned: MinimalAssigned,
  branchLeader: boolean,
): boolean {
  if (branchLeader) return true;
  return hasOversightOnBranch(oversight, org, userEmail, assigned);
}
