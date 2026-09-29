import {
  canReviewPreconvention,
  canViewAllConventions,
  canViewReferentConventions,
} from "@/app/lib/stage-access";
import { classKey } from "@/app/lib/stage-referents-config";
import type { StageWatcherAssignment } from "@/app/lib/stage-watchers-config";
import { conventionMatchesWatcherAssignments } from "@/app/lib/stage-watchers-config";
import type { StageConvention } from "@/app/lib/stage-types";

function conventionMatchesReferent(
  convention: StageConvention,
  userEmail: string,
  userId?: string,
): boolean {
  const refEmail = convention.teacherReferent.email?.trim().toLowerCase();
  const email = userEmail.trim().toLowerCase();
  if (refEmail && email && refEmail === email) return true;
  if (userId && convention.teacherReferent.userId === userId) return true;
  return false;
}

function conventionMatchesPrincipalClass(
  convention: StageConvention,
  principalClassNames: string[],
): boolean {
  if (principalClassNames.length === 0) return false;
  const classK = classKey(convention.student.className);
  return principalClassNames.some((c) => classKey(c) === classK);
}

/**
 * Visibilité d'une convention :
 * - admin / direction / surveillant : tout
 * - watchers : périmètre affectation
 * - professeur principal : toute sa classe
 * - professeur référent : uniquement les stagiaires où il est teacherReferent
 */
export function conventionVisibleToUser(
  convention: StageConvention,
  roles: string[],
  userEmail: string,
  userId?: string,
  /** Classes où l'utilisateur est PP (pas les seules affectations référent). */
  principalClassNames?: string[],
  watcherAssignments?: StageWatcherAssignment[],
): boolean {
  if (canViewAllConventions(roles) || canReviewPreconvention(roles)) return true;
  if (watcherAssignments && watcherAssignments.length > 0) {
    if (conventionMatchesWatcherAssignments(convention, watcherAssignments)) return true;
  }
  if (canViewReferentConventions(roles)) {
    if (conventionMatchesReferent(convention, userEmail, userId)) return true;
    if (principalClassNames?.length) {
      return conventionMatchesPrincipalClass(convention, principalClassNames);
    }
    return false;
  }
  // CPE / accueil : uniquement via affectations watchers (classe ou élève).
  if (roles.includes("cpe") || roles.includes("accueil")) {
    return false;
  }
  return false;
}

export function canPurgeStages(roles: string[]) {
  return canReviewPreconvention(roles);
}
