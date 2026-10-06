import { NextResponse } from "next/server";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { canReviewPreconvention } from "@/app/lib/stage-access";
import { userCanAssignStageReferentForClass } from "@/app/lib/stage-referents-config";
import { notifyReferentAssignmentsForClass } from "@/app/lib/stage-referent-students";
import { currentStageSchoolYear } from "@/app/lib/stage-types";

export async function POST(req: Request) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    const body = (await req.json()) as Record<string, unknown>;
    const className = String(body.className ?? "").trim();
    const schoolYear = String(body.schoolYear ?? "").trim() || currentStageSchoolYear();
    const teacherExternalUserId = String(body.teacherExternalUserId ?? "").trim() || undefined;
    if (!className) {
      return NextResponse.json({ error: "Classe requise." }, { status: 400 });
    }

    const allowed =
      canReviewPreconvention(roles) ||
      (await userCanAssignStageReferentForClass(gate.ctx.userId, className, schoolYear));
    if (!allowed) {
      return NextResponse.json(
        { error: "Seul le professeur principal de la classe (ou l'administratif) peut notifier les référents." },
        { status: 403 },
      );
    }

    const mail = await notifyReferentAssignmentsForClass({
      className,
      schoolYear,
      teacherExternalUserId,
    });

    return NextResponse.json({ success: true, ...mail });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
