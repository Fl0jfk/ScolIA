import { getJson, putJson } from "@/app/lib/s3-storage";
import {
  parseRequestsOrg,
  type RequestsOrgConfig,
  type RequestsRoutingConfig,
  type RequestServiceUnit,
  type StaffDirectoryRow,
} from "@/app/lib/app-config-schemas";
import { saveStaffDirectory } from "@/app/lib/app-config";
import { normalizeRequestEmail } from "@/app/lib/requests-board";
import { invalidateRequestRoutesCache } from "@/app/lib/requests-routes-cache";
import { getRequestsRoutingConfig } from "@/app/lib/requests-routing-config";

const ORG_KEY = "settings/requests-org.json";
const CACHE_MS = 45_000;

let cache: { at: number; config: RequestsOrgConfig } | null = null;

function uid(prefix: string) {
  return `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`;
}

export function defaultRequestsOrg(): RequestsOrgConfig {
  return {
    version: 1,
    globalOversightUnitIds: ["direction_generale"],
    units: [
      {
        id: "direction_generale",
        label: "Direction générale",
        parentUnitId: null,
        managerEmails: [],
        memberEmails: [],
        taskIds: ["direction_ecole", "direction_college", "direction_lycee"],
        canDelegateToChildUnits: true,
        active: true,
      },
      {
        id: "comptabilite",
        label: "Comptabilité",
        parentUnitId: null,
        managerEmails: [],
        memberEmails: [],
        taskIds: ["comptabilite"],
        canDelegateToChildUnits: true,
        active: true,
      },
      {
        id: "maintenance",
        label: "Maintenance",
        parentUnitId: null,
        managerEmails: [],
        memberEmails: [],
        taskIds: ["maintenance"],
        canDelegateToChildUnits: true,
        active: true,
      },
      {
        id: "cpe_college",
        label: "CPE collège",
        parentUnitId: null,
        managerEmails: [],
        memberEmails: [],
        taskIds: ["cpe_3e4e", "cpe_5e6e"],
        canDelegateToChildUnits: true,
        active: true,
      },
      {
        id: "surveillants_college",
        label: "Surveillants collège",
        parentUnitId: "cpe_college",
        managerEmails: [],
        memberEmails: [],
        taskIds: ["cpe_3e4e", "cpe_5e6e"],
        canDelegateToChildUnits: false,
        active: true,
      },
      {
        id: "cpe_lycee",
        label: "CPE lycée",
        parentUnitId: null,
        managerEmails: [],
        memberEmails: [],
        taskIds: ["cpe_lycee"],
        canDelegateToChildUnits: true,
        active: true,
      },
      {
        id: "surveillants_lycee",
        label: "Surveillants lycée",
        parentUnitId: "cpe_lycee",
        managerEmails: [],
        memberEmails: [],
        taskIds: ["cpe_lycee"],
        canDelegateToChildUnits: false,
        active: true,
      },
      {
        id: "accueil",
        label: "Accueil",
        parentUnitId: null,
        managerEmails: [],
        memberEmails: [],
        taskIds: ["accueil"],
        canDelegateToChildUnits: true,
        active: true,
      },
      {
        id: "vie_scolaire",
        label: "Vie scolaire / infirmerie",
        parentUnitId: null,
        managerEmails: [],
        memberEmails: [],
        taskIds: ["vie_scolaire_infirmerie"],
        canDelegateToChildUnits: true,
        active: true,
      },
      {
        id: "corbeille_etablissement",
        label: "Corbeille établissement",
        parentUnitId: null,
        managerEmails: [],
        memberEmails: [],
        taskIds: ["corbeille"],
        canDelegateToChildUnits: false,
        active: true,
      },
    ],
  };
}

export function invalidateRequestsOrgCache() {
  cache = null;
}

export async function getRequestsOrgConfig(): Promise<RequestsOrgConfig> {
  if (cache && Date.now() - cache.at < CACHE_MS) return cache.config;
  const raw = await getJson<{ data?: unknown }>(ORG_KEY);
  const config = raw?.data ? parseRequestsOrg(raw.data) : defaultRequestsOrg();
  cache = { at: Date.now(), config };
  return config;
}

function getActiveAssignments(routing: RequestsRoutingConfig) {
  const activeTaskIds = new Set(routing.tasks.filter((t) => t.active).map((t) => t.id));
  return routing.assignments.filter((a) => a.active && activeTaskIds.has(a.taskId));
}

/** Fusionne routage + organisation services → staff-directory (leaders + executors). */
export function mergeStaffDirectoryFromRoutingAndOrg(
  routing: RequestsRoutingConfig,
  org: RequestsOrgConfig,
): StaffDirectoryRow[] {
  const rows: StaffDirectoryRow[] = [];
  const seen = new Set<string>();

  const push = (email: string, branchId: string, role: "leader" | "executor") => {
    const e = email.trim().toLowerCase();
    if (!e) return;
    const key = `${e}::${branchId}::${role}`;
    if (seen.has(key)) return;
    seen.add(key);
    rows.push({ email: e, branchId, role });
  };

  const activeUnits = org.units.filter((u) => u.active);

  for (const unit of activeUnits) {
    for (const taskId of unit.taskIds) {
      for (const email of unit.managerEmails) {
        push(email, taskId, "leader");
      }
      for (const email of unit.memberEmails) {
        push(email, taskId, "executor");
      }
    }
  }

  for (const a of getActiveAssignments(routing)) {
    const hasLeader = rows.some(
      (r) => r.branchId === a.taskId && r.role === "leader" && r.email === a.email.toLowerCase(),
    );
    if (!hasLeader) {
      push(a.email, a.taskId, "leader");
    }
  }

  for (const d of routing.directionQueues.filter((q) => q.active)) {
    push(d.email, d.id, "leader");
  }

  const corbeille = getActiveAssignments(routing).find((a) => a.taskId === "corbeille");
  if (corbeille) {
    push(corbeille.email, "corbeille", "leader");
  }

  return rows;
}

export async function syncStaffDirectoryFromRequestsConfig(
  routing?: RequestsRoutingConfig,
  org?: RequestsOrgConfig,
): Promise<void> {
  const [r, o] = await Promise.all([
    routing ?? getRequestsRoutingConfig(),
    org ?? getRequestsOrgConfig(),
  ]);
  await saveStaffDirectory(mergeStaffDirectoryFromRoutingAndOrg(r, o));
  invalidateRequestRoutesCache();
}

export async function saveRequestsOrgConfig(config: RequestsOrgConfig): Promise<void> {
  const parsed = parseRequestsOrg(config);
  await putJson(ORG_KEY, { version: 1, updatedAt: new Date().toISOString(), data: parsed });
  invalidateRequestsOrgCache();
  await syncStaffDirectoryFromRequestsConfig(undefined, parsed);
}

export function getActiveUnits(org: RequestsOrgConfig): RequestServiceUnit[] {
  return org.units.filter((u) => u.active);
}

export function findUnitsForBranch(org: RequestsOrgConfig, branchId: string): RequestServiceUnit[] {
  const b = branchId.trim();
  return getActiveUnits(org).filter((u) => u.taskIds.includes(b));
}

export function getChildUnits(org: RequestsOrgConfig, parentId: string): RequestServiceUnit[] {
  return getActiveUnits(org).filter((u) => u.parentUnitId === parentId);
}

export function getDescendantUnitIds(org: RequestsOrgConfig, parentId: string): string[] {
  const out: string[] = [];
  const walk = (id: string) => {
    for (const child of getChildUnits(org, id)) {
      out.push(child.id);
      walk(child.id);
    }
  };
  walk(parentId);
  return out;
}

export function isManagerOfUnit(org: RequestsOrgConfig, unitId: string, email: string): boolean {
  const u = normalizeRequestEmail(email);
  const unit = getActiveUnits(org).find((x) => x.id === unitId);
  if (!unit) return false;
  return unit.managerEmails.map(normalizeRequestEmail).includes(u);
}

export function isGlobalOversightManager(org: RequestsOrgConfig, email: string): boolean {
  const u = normalizeRequestEmail(email);
  for (const gid of org.globalOversightUnitIds) {
    if (isManagerOfUnit(org, gid, u)) return true;
  }
  return false;
}

export function collectDelegateEmailsFromOrg(
  org: RequestsOrgConfig,
  branchId: string,
  leaderEmail: string,
  poolEmails: string[],
): string[] {
  const leader = normalizeRequestEmail(leaderEmail);
  const targets = new Set<string>();

  for (const e of poolEmails.map(normalizeRequestEmail)) {
    if (e && e !== leader) targets.add(e);
  }

  const branchUnits = findUnitsForBranch(org, branchId);

  for (const unit of branchUnits) {
    if (!isManagerOfUnit(org, unit.id, leader)) continue;

    for (const email of unit.memberEmails) {
      const n = normalizeRequestEmail(email);
      if (n && n !== leader) targets.add(n);
    }

    if (unit.canDelegateToChildUnits) {
      for (const childId of getDescendantUnitIds(org, unit.id)) {
        const child = getActiveUnits(org).find((u) => u.id === childId);
        if (!child) continue;
        const applies = child.taskIds.length === 0 || child.taskIds.includes(branchId);
        if (!applies) continue;
        for (const email of child.memberEmails) {
          const n = normalizeRequestEmail(email);
          if (n && n !== leader) targets.add(n);
        }
        for (const email of child.managerEmails) {
          const n = normalizeRequestEmail(email);
          if (n && n !== leader) targets.add(n);
        }
      }
    }
  }

  if (isGlobalOversightManager(org, leader)) {
    for (const unit of getActiveUnits(org)) {
      for (const email of [...unit.managerEmails, ...unit.memberEmails]) {
        const n = normalizeRequestEmail(email);
        if (n && n !== leader) targets.add(n);
      }
    }
  }

  return [...targets].sort((a, b) => a.localeCompare(b, "fr"));
}

export function newRequestServiceUnit(label = "Nouveau service"): RequestServiceUnit {
  return {
    id: uid("unit"),
    label,
    parentUnitId: null,
    managerEmails: [],
    memberEmails: [],
    taskIds: [],
    canDelegateToChildUnits: true,
    active: true,
  };
}
