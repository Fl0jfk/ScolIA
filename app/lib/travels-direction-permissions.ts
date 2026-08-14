/** Aligné sur la logique « canSign » de la fiche voyage (direction par établissement). */

import type { ClerkLikeUser } from "@/app/lib/clerk-user-types";

const norm = (v: string) =>
  v.normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase().replace(/[_\s-]+/g, "");

export function canSignTravelsDirectionForEtab(
  user: ClerkLikeUser | null | undefined,
  etablissement: string | null | undefined,
): boolean {
  if (!user?.publicMetadata) return false;
  const rawRoles = user.publicMetadata.role;
  const userRoles: string[] = Array.isArray(rawRoles) ? (rawRoles as string[]) : rawRoles ? [String(rawRoles)] : [];
  const normalizedRoles = userRoles.map((r) => norm(String(r)));
  const isDirectionLycee = userRoles.includes("direction_lycee");
  const isDirectionCollege = userRoles.includes("direction collège");
  const isDirectionEcole = userRoles.includes("direction école");
  const isEcoleDir = normalizedRoles.some(
    (r) => r.includes("directionecole") || r.includes("directionecol") || (r.includes("direction") && r.includes("ecole"))
  );
  const isCollegeDir = normalizedRoles.some((r) => r.includes("directioncollege") || (r.includes("direction") && r.includes("college")));
  const isLyceeDir = normalizedRoles.some((r) => r.includes("directionlycee") || (r.includes("direction") && r.includes("lycee")));
  const etabForSign = etablissement || "";
  if (etabForSign === "École") return isDirectionEcole || isEcoleDir;
  if (etabForSign === "Collège") return isDirectionCollege || isCollegeDir;
  if (etabForSign === "Lycée") return isDirectionLycee || isLyceeDir;
  return isDirectionLycee || isLyceeDir;
}

export function isTripOwner(tripOwnerId: string | null | undefined, clerkUserId: string | null | undefined): boolean {
  return Boolean(tripOwnerId && clerkUserId && tripOwnerId === clerkUserId);
}

/** ownerId Clerk en priorité, repli sur ownerName pour dossiers anciens. */
export function isTripOwnerOrCreator(
  trip: { ownerId?: string | null; ownerName?: string | null },
  user: ClerkLikeUser | null | undefined,
): boolean {
  if (!user) return false;
  if (isTripOwner(trip.ownerId, user.id)) return true;
  if (trip.ownerName && user.fullName && trip.ownerName.trim() === user.fullName.trim()) return true;
  return false;
}

/** Statuts workflow autorisés lors d'une réouverture depuis « Finalisé » (VALIDE). */
const TRAVELS_REOPEN_FROM_VALIDE_STATUSES = [
  "EN_ATTENTE_DIR_INITIAL",
  "PROF_LOGISTICS",
  "EN_ATTENTE_BUS_SIGNATURE",
  "EN_ATTENTE_COMPTA",
  "EN_ATTENTE_DIR_FINAL",
] as const;

export function isValidTravelsReopenFromValideStatus(status: string): boolean {
  return (TRAVELS_REOPEN_FROM_VALIDE_STATUSES as readonly string[]).includes(status);
}
