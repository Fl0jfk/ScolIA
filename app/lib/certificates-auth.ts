import "server-only";

import { getClerkUserRoles } from "@/app/lib/clerk-users";
import { hasGlobalAdminRole, hasRole } from "@/app/lib/intranet-role-utils";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import {
  CERTIFICATE_DIRECTION_ROLE_BY_SECTEUR,
  type CertificateProgram,
  type StudentAward,
} from "@/app/lib/certificates-types";
import type { ClerkActor } from "@/app/lib/clerk-user-types";

function rolesOf(user: ClerkActor | null | undefined): string[] {
  return rolesFromUserLike(user);
}

export function canAccessCertificatesModule(user: ClerkActor | null | undefined): boolean {
  const roles = rolesOf(user);
  if (hasGlobalAdminRole(roles)) return true;
  return (
    hasRole(roles, "professeur") ||
    hasRole(roles, "administratif") ||
    hasRole(roles, "direction_ecole") ||
    hasRole(roles, "direction_college") ||
    hasRole(roles, "direction_lycee")
  );
}

export function isProgramOwner(program: CertificateProgram, userId: string): boolean {
  return program.ownerId === userId;
}

export function isProgramCollaborator(program: CertificateProgram, userId: string): boolean {
  return program.collaboratorIds.includes(userId);
}

export function canViewProgram(program: CertificateProgram, userId: string): boolean {
  return isProgramOwner(program, userId) || isProgramCollaborator(program, userId);
}

export function canEditProgramTitle(program: CertificateProgram, userId: string): boolean {
  return isProgramOwner(program, userId);
}

export function canDeleteProgram(program: CertificateProgram, userId: string): boolean {
  return isProgramOwner(program, userId);
}

export function canManageProgramCollaborators(program: CertificateProgram, userId: string): boolean {
  return isProgramOwner(program, userId);
}

export function canWorkOnProgram(program: CertificateProgram, userId: string): boolean {
  return canViewProgram(program, userId);
}

export function eligibleSignatoryIds(program: CertificateProgram): string[] {
  return [program.ownerId, ...program.collaboratorIds].filter(
    (id, i, arr) => id && arr.indexOf(id) === i,
  );
}

export function canEditAwardSignatories(award: StudentAward, program: CertificateProgram, userId: string): boolean {
  if (award.status !== "draft") return false;
  return canManageAwardSignatoriesBase(program, userId, award.addedBy);
}

/** Retirer / ajouter des profs signataires tant que la direction n'a pas signé. */
export function canEditAwardSignatoriesDuringSigning(
  award: StudentAward,
  program: CertificateProgram,
  userId: string,
): boolean {
  if (award.directionSignature) return false;
  if (!["submitted", "prof_signed"].includes(award.status)) return false;
  return canManageAwardSignatoriesBase(program, userId, award.addedBy);
}

function canManageAwardSignatoriesBase(
  program: CertificateProgram,
  userId: string,
  addedBy: string,
): boolean {
  return (
    isProgramOwner(program, userId) ||
    isProgramCollaborator(program, userId) ||
    addedBy === userId
  );
}

export function canManageAwardSignatories(
  award: StudentAward,
  program: CertificateProgram,
  userId: string,
): boolean {
  return (
    canEditAwardSignatories(award, program, userId) ||
    canEditAwardSignatoriesDuringSigning(award, program, userId)
  );
}

export function canEditAwardLines(
  award: StudentAward,
  program: CertificateProgram,
  userId: string,
): boolean {
  if (award.status !== "draft") return false;
  return canWorkOnProgram(program, userId);
}

export function canSubmitAward(
  award: StudentAward,
  program: CertificateProgram,
  userId: string,
): boolean {
  if (award.status !== "draft") return false;
  return isProgramOwner(program, userId) || award.addedBy === userId;
}

export function isDesignatedSignatory(award: StudentAward, userId: string): boolean {
  return award.designatedSignatories.some((s) => s.clerkUserId === userId);
}

export function canSignAwardAsProf(award: StudentAward, userId: string): boolean {
  if (award.status !== "submitted" && award.status !== "prof_signed") return false;
  const sig = award.designatedSignatories.find((s) => s.clerkUserId === userId);
  return Boolean(sig && sig.status === "pending");
}

export function requiredDirectionRoleForAward(award: StudentAward): string {
  return CERTIFICATE_DIRECTION_ROLE_BY_SECTEUR[award.student.secteur];
}

function awardReadyForDirectionSignature(award: StudentAward): boolean {
  if (award.directionSignature) return false;
  const allProfsSigned =
    award.designatedSignatories.length > 0 &&
    award.designatedSignatories.every((s) => s.status === "signed");
  if (!allProfsSigned) return false;
  return ["submitted", "prof_signed", "direction_signed"].includes(award.status);
}

/** Vérification stricte : le slug exact doit être présent dans les rôles Clerk. */
export function canSignAwardAsDirectionWithRoles(roles: string[], award: StudentAward): boolean {
  if (!awardReadyForDirectionSignature(award)) return false;
  const directionRole = requiredDirectionRoleForAward(award);
  return roles.includes(directionRole);
}

/** Source de vérité : rôles relus depuis l'API Clerk (pas le JWT de session). */
export async function canSignAwardAsDirectionForUserId(
  userId: string,
  award: StudentAward,
): Promise<boolean> {
  if (!userId.trim()) return false;
  const roles = await getClerkUserRoles(userId);
  return canSignAwardAsDirectionWithRoles(roles, award);
}

export function awardAwaitingDirectionSignature(award: StudentAward): boolean {
  if (award.directionSignature) return false;
  return (
    award.designatedSignatories.length > 0 &&
    award.designatedSignatories.every((s) => s.status === "signed")
  );
}

export function canViewAward(
  award: StudentAward,
  program: CertificateProgram,
  userId: string,
  user?: ClerkActor | null,
): boolean {
  if (canViewProgram(program, userId)) return true;
  if (isDesignatedSignatory(award, userId)) return true;
  const roles = rolesOf(user);
  if (hasRole(roles, "administratif") || hasGlobalAdminRole(roles)) return true;
  return false;
}

export function canDownloadAwardPdf(
  award: StudentAward,
  program: CertificateProgram,
  userId: string,
  user?: ClerkActor | null,
): boolean {
  if (award.status !== "issued") return false;
  return canViewAward(award, program, userId, user);
}
