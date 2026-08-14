import {
  canReviewPreconvention,
  canViewAllConventions,
} from "@/app/lib/stage-access";
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

export function conventionVisibleToUser(
  convention: StageConvention,
  roles: string[],
  userEmail: string,
  userId?: string,
): boolean {
  if (canViewAllConventions(roles)) return true;
  if (canViewReferentConventions(roles)) {
    return conventionMatchesReferent(convention, userEmail, userId);
  }
  if (canReviewPreconvention(roles)) return true;
  return false;
}

export function canPurgeStages(roles: string[]) {
  return canReviewPreconvention(roles);
}
