import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import {
  canModerateOffers,
  canReviewPreconvention,
  canViewAllConventions,
  canViewReferentConventions,
  canFileConventionToOneDrive,
  resolveStageViewerRole,
} from "@/app/lib/stage-access";
import { conventionVisibleToUser } from "@/app/lib/stage-referent";
import { listPendingSignaturesForUser } from "@/app/lib/stage-pending-signatures";
import {
  getConventionsIndex,
  getOffersIndex,
  getStageConvention,
  getStageOffer,
} from "@/app/lib/stage-storage";
import { STAGE_CONVENTION_STATUS_LABELS, STAGE_OFFER_KIND_LABELS } from "@/app/lib/stage-types";

export async function GET() {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    const viewer = resolveStageViewerRole(roles);
    if (!viewer) {
      return NextResponse.json({ error: "Accès réservé." }, { status: 403 });
    }

    const [offers, conventionsIndex] = await Promise.all([getOffersIndex(), getConventionsIndex()]);
    const allConventions = await Promise.all(
      conventionsIndex.map((e) => getStageConvention(e.id)),
    );
    const userEmail = user?.primaryEmailAddress?.emailAddress?.trim().toLowerCase() || "";
    const conventions = allConventions
      .filter((c): c is NonNullable<typeof c> => Boolean(c))
      .filter((c) => conventionVisibleToUser(c, roles, userEmail, gate.ctx.userId));

    const pendingOffers = offers.filter((o) => o.status === "pending");
    const adminQueue = conventions.filter(
      (c) =>
        c.status === "admin_review" ||
        c.status === "preconvention_submitted" ||
        c.status === "convention_deposited",
    );
    const signaturesPending = conventions.filter((c) => c.status === "signatures_pending");
    const referentOnly = canViewReferentConventions(roles) && !canViewAllConventions(roles);
    const myPendingSignatures = await listPendingSignaturesForUser(
      conventions,
      userEmail,
      gate.ctx.userId,
      roles,
    );

    return NextResponse.json({
      viewer,
      permissions: {
        canModerateOffers: canModerateOffers(roles),
        canReviewPreconvention: canReviewPreconvention(roles),
        canViewAllConventions: canViewAllConventions(roles),
        canViewReferentConventions: canViewReferentConventions(roles),
        canDepositOffer: roles.includes("parent"),
        canFileToOneDrive: canFileConventionToOneDrive(roles),
        canPurge: canReviewPreconvention(roles),
        canManageReferents: canReviewPreconvention(roles),
        referentOnly,
        canViewClassRoster: canViewReferentConventions(roles) || canViewAllConventions(roles),
      },
      counts: {
        offers: offers.length,
        pendingOffers: pendingOffers.length,
        conventions: conventions.length,
        adminQueue: adminQueue.length,
        signaturesPending: signaturesPending.length,
        myPendingSignatures: myPendingSignatures.length,
      },
      myPendingSignatures,
      pendingOffers: pendingOffers.slice(0, 20),
      adminQueue: adminQueue.slice(0, 20),
      signaturesPending: signaturesPending.slice(0, 20),
      conventions: conventions.slice(0, 100).map((c) => ({
        id: c.id,
        studentName: `${c.student.firstName} ${c.student.lastName}`.trim(),
        className: c.student.className,
        companyName: c.company.name,
        status: c.status,
        periodStart: c.schedule.periodStart,
        periodEnd: c.schedule.periodEnd,
      })),
      labels: {
        offerKinds: STAGE_OFFER_KIND_LABELS,
        conventionStatuses: STAGE_CONVENTION_STATUS_LABELS,
      },
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
