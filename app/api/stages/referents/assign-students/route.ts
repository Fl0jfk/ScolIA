import { NextResponse } from "next/server";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { canReviewPreconvention } from "@/app/lib/stage-access";
import { userCanAssignStageReferentForClass } from "@/app/lib/stage-referents-config";
import {
  notifyReferentAssignmentsForClass,
  upsertStudentReferentAssignments,
} from "@/app/lib/stage-referent-students";
import { currentStageSchoolYear } from "@/app/lib/stage-types";
import { stageActorFirstName } from "@/app/lib/stage-actor-name";

export async function POST(req: Request) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    const body = (await req.json()) as Record<string, unknown>;
    const className = String(body.className ?? "").trim();
    const schoolYear = String(body.schoolYear ?? "").trim() || currentStageSchoolYear();
    if (!className) {
      return NextResponse.json({ error: "Classe requise." }, { status: 400 });
    }

    const allowed =
      canReviewPreconvention(roles) ||
      (await userCanAssignStageReferentForClass(gate.ctx.userId, className, schoolYear));
    if (!allowed) {
      return NextResponse.json(
        { error: "Seul le professeur principal de la classe (ou l'administratif) peut affecter un référent." },
        { status: 403 },
      );
    }

    const teacherRaw = body.teacher && typeof body.teacher === "object" ? (body.teacher as Record<string, unknown>) : {};
    const teacher = {
      externalUserId: String(teacherRaw.externalUserId ?? "").trim(),
      name: String(teacherRaw.name ?? "").trim(),
      email: String(teacherRaw.email ?? "").trim().toLowerCase(),
    };
    const studentsRaw = Array.isArray(body.students) ? body.students : [];
    const students = studentsRaw
      .filter((row): row is Record<string, unknown> => Boolean(row) && typeof row === "object")
      .map((row) => ({
        key: String(row.key ?? "").trim(),
        eleveId: String(row.eleveId ?? "").trim() || undefined,
        ine: String(row.ine ?? "").trim() || undefined,
        studentName: String(row.studentName ?? "").trim(),
      }));

    const result = await upsertStudentReferentAssignments({
      className,
      schoolYear,
      updatedBy: stageActorFirstName(user),
      teacher,
      students,
    });

    const notify = body.notify !== false;
    const mail = notify
      ? await notifyReferentAssignmentsForClass({
          className,
          schoolYear,
          teacherExternalUserId: teacher.externalUserId,
        })
      : { sent: 0, skipped: 0, results: [] };

    return NextResponse.json({
      success: true,
      updatedConventions: result.updatedConventions,
      assigned: students.length,
      notified: mail.sent > 0,
      mail,
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
