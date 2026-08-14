import { loadAppConfig, invalidateAppConfigCache } from "@/app/lib/app-config";
import type { StaffDirectoryRow as ConfigStaffRow } from "@/app/lib/app-config-schemas";
import { normalizeRequestEmail } from "@/app/lib/requests-board";

type StaffRequestBranchId =
  | "corbeille"
  | "maintenance"
  | "admin_ecole"
  | "admin_college"
  | "admin_lycee"
  | "cpe_lycee"
  | "cpe_3e4e"
  | "cpe_5e6e"
  | "vie_scolaire_infirmerie"
  | "accueil"
  | "comptabilite"
  | "direction_ecole"
  | "direction_college"
  | "direction_lycee";

type StaffDirectoryRow = {
  email: string;
  branchId: StaffRequestBranchId | string;
  role: "leader" | "executor";
  validUntil?: string;
};

let cachedRows: StaffDirectoryRow[] | null = null;
let cacheAt = 0;
const CACHE_MS = 45_000;

function invalidateStaffDirectoryCache() {
  cachedRows = null;
  cacheAt = 0;
  invalidateAppConfigCache();
}

export async function loadStaffDirectoryRows(): Promise<StaffDirectoryRow[]> {
  if (cachedRows && Date.now() - cacheAt < CACHE_MS) return cachedRows;
  const config = await loadAppConfig();
  cachedRows = config.staffDirectory.map((r) => ({
    email: r.email,
    branchId: r.branchId,
    role: r.role,
    validUntil: r.validUntil,
  }));
  cacheAt = Date.now();
  return cachedRows;
}

function isStaffRowActive(row: StaffDirectoryRow, now = new Date()): boolean {
  const v = row.validUntil?.trim();
  if (!v) return true;
  const end = new Date(v);
  if (Number.isNaN(end.getTime())) return true;
  return now.getTime() <= end.getTime();
}

function activeRows(rows: StaffDirectoryRow[], now = new Date()) {
  return rows.filter((r) => isStaffRowActive(r, now));
}

export function getStaffLeadersForBranchFromRows(
  rows: StaffDirectoryRow[],
  branchId: string,
  now = new Date(),
): string[] {
  const b = branchId.trim();
  return [
    ...new Set(
      activeRows(rows, now)
        .filter((r) => r.branchId === b && r.role === "leader")
        .map((r) => normalizeRequestEmail(r.email))
        .filter(Boolean),
    ),
  ];
}

export function getStaffExecutorsForBranchFromRows(
  rows: StaffDirectoryRow[],
  branchId: string,
  now = new Date(),
): string[] {
  const b = branchId.trim();
  return [
    ...new Set(
      activeRows(rows, now)
        .filter((r) => r.branchId === b && r.role === "executor")
        .map((r) => normalizeRequestEmail(r.email))
        .filter(Boolean),
    ),
  ];
}

async function getStaffLeadersForBranch(branchId: string, now = new Date()): Promise<string[]> {
  const rows = await loadStaffDirectoryRows();
  return getStaffLeadersForBranchFromRows(rows, branchId, now);
}

async function getStaffExecutorsForBranch(branchId: string, now = new Date()): Promise<string[]> {
  const rows = await loadStaffDirectoryRows();
  return getStaffExecutorsForBranchFromRows(rows, branchId, now);
}

async function getStaffPoolForBranch(branchId: string, now = new Date()): Promise<string[]> {
  const leaders = await getStaffLeadersForBranch(branchId, now);
  const execs = await getStaffExecutorsForBranch(branchId, now);
  return [...new Set([...leaders, ...execs].filter(Boolean))];
}

async function isStaffLeaderForBranch(
  branchId: string,
  actorEmail: string,
  now = new Date(),
): Promise<boolean> {
  if (!actorEmail) return false;
  return (await getStaffLeadersForBranch(branchId, now)).includes(normalizeRequestEmail(actorEmail));
}

export async function isStaffInBranchPool(
  branchId: string,
  actorEmail: string,
  now = new Date(),
): Promise<boolean> {
  if (!actorEmail) return false;
  return (await getStaffPoolForBranch(branchId, now)).includes(normalizeRequestEmail(actorEmail));
}

async function getAllStaffEmailsFromDirectory(now = new Date()): Promise<string[]> {
  const rows = await loadStaffDirectoryRows();
  const s = new Set<string>();
  for (const r of activeRows(rows, now)) {
    if (r.branchId === "corbeille") continue;
    s.add(normalizeRequestEmail(r.email));
  }
  return [...s];
}

export async function isListedAsRequestsStaff(actorEmail: string, now = new Date()): Promise<boolean> {
  if (!actorEmail) return false;
  const rows = await loadStaffDirectoryRows();
  const u = normalizeRequestEmail(actorEmail);
  return activeRows(rows, now).some(
    (r) => r.branchId !== "corbeille" && normalizeRequestEmail(r.email) === u,
  );
}

export async function getFirstBranchForStaffEmailFromDirectory(
  actorEmail: string,
  now = new Date(),
): Promise<string | null> {
  const rows = await loadStaffDirectoryRows();
  const u = normalizeRequestEmail(actorEmail);
  for (const r of activeRows(rows, now)) {
    if (r.branchId === "corbeille") continue;
    if (normalizeRequestEmail(r.email) === u) return r.branchId;
  }
  return null;
}

function configRowsToStaffDirectory(rows: ConfigStaffRow[]): StaffDirectoryRow[] {
  return rows.map((r) => ({
    email: r.email,
    branchId: r.branchId,
    role: r.role,
    validUntil: r.validUntil,
  }));
}
