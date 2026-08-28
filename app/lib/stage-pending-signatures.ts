import type { StageConvention, StageSignature } from "@/app/lib/stage-types";
import { STAGE_SIGNER_ROLE_LABELS } from "@/app/lib/stage-types";
import { INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";
import { resolveStagesDirectionEmail } from "@/app/lib/stage-config";

export type PendingStageSignature = {
  conventionId: string;
  signatureId: string;
  role: string;
  roleLabel: string;
  studentName: string;
  className: string;
  companyName: string;
  periodStart: string;
  periodEnd: string;
  signLink: string;
  validatedAt?: string;
};

function hasDirectionRole(roles: string[]): boolean {
  return roles.some((r) =>
    INTRANET_DIRECTION_SLUGS.includes(r as (typeof INTRANET_DIRECTION_SLUGS)[number]),
  );
}

async function signatureAwaitingUser(
  sig: StageSignature,
  convention: StageConvention,
  userEmail: string,
  userId?: string,
  roles: string[] = [],
): Promise<boolean> {
  if (sig.status !== "en_attente" || !sig.signToken) return false;

  const email = userEmail.trim().toLowerCase();
  const sigEmail = sig.signEmail?.trim().toLowerCase();
  if (sigEmail && email && sigEmail === email) return true;

  if (sig.role === "professeur_referent") {
    if (userId && convention.teacherReferent.userId === userId) return true;
    const refEmail = convention.teacherReferent.email?.trim().toLowerCase();
    if (refEmail && email && refEmail === email) return true;
  }

  if (sig.role === "direction" && hasDirectionRole(roles)) {
    const directionEmail = (await resolveStagesDirectionEmail(convention.student.level))?.toLowerCase();
    if (!directionEmail || !email || directionEmail === email) return true;
    if (sigEmail && email && sigEmail === email) return true;
  }

  return false;
}

/** Conventions en attente de signature pour l'utilisateur connecté. */
export async function listPendingSignaturesForUser(
  conventions: StageConvention[],
  userEmail: string,
  userId?: string,
  roles: string[] = [],
): Promise<PendingStageSignature[]> {
  const out: PendingStageSignature[] = [];

  for (const c of conventions) {
    if (c.status !== "signatures_pending") continue;
    for (const sig of c.signatures) {
      if (!(await signatureAwaitingUser(sig, c, userEmail, userId, roles))) continue;
      out.push({
        conventionId: c.id,
        signatureId: sig.id,
        role: sig.role,
        roleLabel: STAGE_SIGNER_ROLE_LABELS[sig.role],
        studentName: `${c.student.firstName} ${c.student.lastName}`.trim(),
        className: c.student.className,
        companyName: c.company.name,
        periodStart: c.schedule.periodStart,
        periodEnd: c.schedule.periodEnd,
        signLink: `/stages/signer?token=${encodeURIComponent(sig.signToken!)}`,
        validatedAt: c.adminReview?.at,
      });
    }
  }

  out.sort((a, b) => (b.validatedAt || "").localeCompare(a.validatedAt || ""));
  return out;
}
