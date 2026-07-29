import {
  canModerateOffers,
  canReviewPreconvention,
  canViewAllConventions,
  canViewReferentConventions,
  resolveStageViewerRole,
} from "@/app/lib/stage-access";
import { conventionVisibleToUser } from "@/app/lib/stage-referent";
import { listPendingSignaturesForUser } from "@/app/lib/stage-pending-signatures";
import {
  getConventionsIndex,
  getOffersIndex,
  getStageConvention,
} from "@/app/lib/stage-storage";
import {
  STAGE_CONVENTION_STATUS_LABELS,
  STAGE_OFFER_KIND_LABELS,
} from "@/app/lib/stage-types";
import type { BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";

export async function handleGetStagesOverview(ctx: BrainToolCtx): Promise<BrainToolResult> {
  if (!ctx.userId) {
    return { ok: false, error: "Connexion requise.", code: "AUTH_REQUIRED" };
  }

  const viewer = resolveStageViewerRole(ctx.roles);
  if (!viewer) {
    return { ok: false, error: "Accès stages réservé.", code: "MODULE_FORBIDDEN" };
  }

  const [offers, conventionsIndex] = await Promise.all([getOffersIndex(), getConventionsIndex()]);
  const allConventions = await Promise.all(conventionsIndex.map((e) => getStageConvention(e.id)));
  const userEmail = (ctx.email || "").trim().toLowerCase();
  const conventions = allConventions
    .filter((c): c is NonNullable<typeof c> => Boolean(c))
    .filter((c) => conventionVisibleToUser(c, ctx.roles, userEmail, ctx.userId!));

  const pendingOffers = offers.filter((o) => o.status === "pending");
  const adminQueue = conventions.filter(
    (c) =>
      c.status === "admin_review" ||
      c.status === "preconvention_submitted" ||
      c.status === "convention_deposited",
  );
  const signaturesPending = conventions.filter((c) => c.status === "signatures_pending");
  const myPendingSignatures = listPendingSignaturesForUser(conventions, userEmail, ctx.userId);

  const recentConventions = conventions.slice(0, 8).map((c) => ({
    id: c.id,
    studentName: `${c.student.firstName} ${c.student.lastName}`.trim(),
    className: c.student.className,
    companyName: c.company.name,
    status: c.status,
    statusLabel: STAGE_CONVENTION_STATUS_LABELS[c.status] || c.status,
    periodStart: c.schedule.periodStart,
    periodEnd: c.schedule.periodEnd,
  }));

  const counts = {
    offers: offers.length,
    pendingOffers: pendingOffers.length,
    conventions: conventions.length,
    adminQueue: adminQueue.length,
    signaturesPending: signaturesPending.length,
    myPendingSignatures: myPendingSignatures.length,
  };

  const parts: string[] = [
    `Vue stages (${viewer}) : ${counts.conventions} convention(s) visible(s)`,
  ];
  if (counts.pendingOffers) parts.push(`${counts.pendingOffers} offre(s) en attente de modération`);
  if (counts.adminQueue) parts.push(`${counts.adminQueue} dossier(s) file admin`);
  if (counts.myPendingSignatures) {
    parts.push(`${counts.myPendingSignatures} signature(s) en attente pour vous`);
  }

  return {
    ok: true,
    data: {
      viewer,
      permissions: {
        canModerateOffers: canModerateOffers(ctx.roles),
        canReviewPreconvention: canReviewPreconvention(ctx.roles),
        canViewAllConventions: canViewAllConventions(ctx.roles),
        canViewReferentConventions: canViewReferentConventions(ctx.roles),
      },
      counts,
      myPendingSignatures: myPendingSignatures.slice(0, 10),
      recentConventions,
      labels: {
        offerKinds: STAGE_OFFER_KIND_LABELS,
        conventionStatuses: STAGE_CONVENTION_STATUS_LABELS,
      },
      ctas: [{ label: "Ouvrir Stages", href: "/stages" }],
    },
    summaryFr: parts.join(" · ") + ".",
  };
}
