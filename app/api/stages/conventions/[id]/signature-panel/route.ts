import { NextResponse } from "next/server";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import {
  canReviewPreconvention,
  canViewReferentConventions,
} from "@/app/lib/stage-access";
import { conventionVisibleToUser } from "@/app/lib/stage-referent";
import { listPrincipalClassesForUser } from "@/app/lib/stage-referents-config";
import {
  getStageWatchersConfig,
  listWatcherAssignmentsForUser,
} from "@/app/lib/stage-watchers-config";
import { getStageConvention } from "@/app/lib/stage-storage";
import { buildSignatureSummary } from "@/app/lib/stage-signature-summary";
import {
  STAGE_CONVENTION_STATUS_LABELS,
  currentStageSchoolYear,
} from "@/app/lib/stage-types";

/**
 * Charge légère pour le volet « signatures en cours » du tableau de bord :
 * pas de matching OneDrive / période / eleveMatch.
 */
export async function GET(_req: Request, ctx: { params: Promise<{ id: string }> }) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    const { id } = await ctx.params;
    const convention = await getStageConvention(id);
    if (!convention) {
      return NextResponse.json({ error: "Convention introuvable." }, { status: 404 });
    }

    const userEmail = user?.primaryEmailAddress?.emailAddress?.trim().toLowerCase() || "";
    const principalClassNames = canViewReferentConventions(roles)
      ? await listPrincipalClassesForUser(gate.ctx.userId)
      : [];
    const watchers = await getStageWatchersConfig(
      convention.schoolYear || currentStageSchoolYear(),
    );
    const watcherAssignments = listWatcherAssignmentsForUser(watchers, gate.ctx.userId);
    if (
      !conventionVisibleToUser(
        convention,
        roles,
        userEmail,
        gate.ctx.userId,
        principalClassNames,
        watcherAssignments,
      )
    ) {
      return NextResponse.json({ error: "Accès réservé." }, { status: 403 });
    }

    const summary = buildSignatureSummary(convention);
    const emailById = new Map(
      convention.signatures.map((s) => [s.id, s.signEmail?.trim() || ""] as const),
    );

    return NextResponse.json({
      id: convention.id,
      status: convention.status,
      statusLabel: STAGE_CONVENTION_STATUS_LABELS[convention.status] || convention.status,
      studentName: `${convention.student.firstName} ${convention.student.lastName}`.trim(),
      className: convention.student.className || "",
      companyName: convention.company.name || "—",
      periodStart: convention.schedule.periodStart || "",
      periodEnd: convention.schedule.periodEnd || "",
      canResend: canReviewPreconvention(roles) && convention.status === "signatures_pending",
      signatureSummary: {
        ...summary,
        items: summary.items.map((item) => ({
          ...item,
          email: emailById.get(item.id) || undefined,
        })),
      },
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
