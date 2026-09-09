import {
  canReviewPreconvention,
  canViewAllConventions,
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

function canViewReferentConventions(roles: string[]) {
  return roles.includes("professeur");
}

function conventionMatchesReferentClass(
  convention: StageConvention,
  referentClassNames: string[],
): boolean {
  if (referentClassNames.length === 0) return false;
  const classK = classKey(convention.student.className);
  return referentClassNames.some((c) => classKey(c) === classK);
}

export function conventionVisibleToUser(
  convention: StageConvention,
  roles: string[],
  userEmail: string,
  userId?: string,
  referentClassNames?: string[],
  watcherAssignments?: StageWatcherAssignment[],
): boolean {
  if (canViewAllConventions(roles)) return true;
  if (canReviewPreconvention(roles)) return true;
  if (watcherAssignments && watcherAssignments.length > 0) {
    if (conventionMatchesWatcherAssignments(convention, watcherAssignments)) return true;
  }
  if (canViewReferentConventions(roles)) {
    if (conventionMatchesReferent(convention, userEmail, userId)) return true;
    if (referentClassNames?.length) {
      return conventionMatchesReferentClass(convention, referentClassNames);
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
