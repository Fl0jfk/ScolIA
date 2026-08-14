type StaffBoardColumn = "CORBEILLE" | "NOUVELLES" | "EN_COURS" | "EN_ATTENTE" | "TERMINEE";

export const LEGACY_ROUTE_TO_BRANCH: Record<string, string> = {
  "maintenance.consommables": "maintenance",
  "maintenance.batiment": "maintenance",
  "maintenance.photocopieur_panne": "accueil",
  "it.poste": "maintenance",
  "comptabilite.facturation": "comptabilite",
  "scolarite.inscriptions_secretariats": "admin_ecole",
  "scolarite.college.direction": "admin_college",
  "scolarite.college.cpe": "cpe_5e6e",
  "scolarite.college.secretariat": "admin_college",
  "scolarite.ecole.secretariat": "admin_ecole",
  "scolarite.lycee.direction": "admin_lycee",
  "vie_scolaire.absences": "vie_scolaire_infirmerie",
  "tri.inconnu": "corbeille",
};

export function normalizeRequestBranchId(routeId?: string, unit?: string): string {
  const raw = (routeId || unit || "").trim();
  if (!raw) return "corbeille";
  if (LEGACY_ROUTE_TO_BRANCH[raw]) return LEGACY_ROUTE_TO_BRANCH[raw]!;
  return raw;
}

export function isCorbeilleBranchId(branchId: string): boolean {
  return branchId === "corbeille" || branchId === "tri.inconnu";
}

type MinimalAssigned = {
  routeId?: string;
  unit: string;
  email: string;
  poolEmails?: string[];
  claimedBy?: { email: string } | null;
};

export function normalizeRequestEmail(e: string) { return e.trim().toLowerCase()}

export function isVisibleOnStaffBoard(
  assigned: MinimalAssigned,
  userEmail: string,
  allStaffEmails: string[],
  viewerIsLeaderOfThisBranch: boolean,
): boolean {
  if (!userEmail) return false;
  const u = normalizeRequestEmail(userEmail);
  const branch = normalizeRequestBranchId(assigned.routeId, assigned.unit);
  const claimed = assigned.claimedBy?.email;
  if (!claimed) {
    if (isCorbeilleBranchId(branch)) { return allStaffEmails.includes(u)}
    const pool = getEffectivePoolEmails(assigned, branch, allStaffEmails);
    return pool.includes(u);
  }
  if (normalizeRequestEmail(claimed) === u) return true;
  return viewerIsLeaderOfThisBranch && !isCorbeilleBranchId(branch);
}

function getEffectivePoolEmails(assigned: MinimalAssigned, branch: string, allStaffEmails: string[]): string[] {
  if (isCorbeilleBranchId(branch)) return allStaffEmails;
  const raw = assigned.poolEmails && assigned.poolEmails.length > 0 ? assigned.poolEmails : [assigned.email];
  return [...new Set(raw.map(normalizeRequestEmail).filter(Boolean))];
}

function isUserInBranchPool( assigned: MinimalAssigned, userEmail: string, allStaffEmails: string[]): boolean {
  const u = normalizeRequestEmail(userEmail);
  const branch = normalizeRequestBranchId(assigned.routeId, assigned.unit);
  return getEffectivePoolEmails(assigned, branch, allStaffEmails).includes(u);
}

export function computeStaffBoardColumn( assigned: MinimalAssigned, status: string, userEmail: string, allStaffEmails: string[], viewerIsLeaderOfThisBranch: boolean): StaffBoardColumn | null {
  const st = status;
  if (st === "TERMINEE") return "TERMINEE";
  if (st === "EN_ATTENTE") return "EN_ATTENTE";
  const branch = normalizeRequestBranchId(assigned.routeId, assigned.unit);
  const claimed = assigned.claimedBy?.email;
  const u = normalizeRequestEmail(userEmail);
  const inPool = isUserInBranchPool(assigned, userEmail, allStaffEmails);
  if (!claimed) {
    if (isCorbeilleBranchId(branch)) return "CORBEILLE";
    if (inPool) return "NOUVELLES";
    return null;
  }
  if (normalizeRequestEmail(claimed) === u) {
    if (st === "NOUVELLE" || st === "EN_COURS") return "EN_COURS";
    return null;
  }
  if (viewerIsLeaderOfThisBranch && !isCorbeilleBranchId(branch) && (st === "NOUVELLE" || st === "EN_COURS")) {
    return "EN_COURS";
  }
  return null;
}