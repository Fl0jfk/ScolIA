import {
  canReviewPreconvention,
  canViewAllConventions,
} from "@/app/lib/stage-access";
import { classKey } from "@/app/lib/stage-referents-config";
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
): boolean {
  if (canViewAllConventions(roles)) return true;
  if (canViewReferentConventions(roles)) {
    if (conventionMatchesReferent(convention, userEmail, userId)) return true;
    if (referentClassNames?.length) {
      return conventionMatchesReferentClass(convention, referentClassNames);
    }
    return false;
  }
  if (canReviewPreconvention(roles)) return true;
  return false;
}

export function canPurgeStages(roles: string[]) {
  return canReviewPreconvention(roles);
}
