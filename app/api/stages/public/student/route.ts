import { NextResponse } from "next/server";
import {
  normalizeConventionInput,
  resolveConventionByStudentToken,
  submitPreconvention,
} from "@/app/lib/stage-workflow";
import { saveStageConvention } from "@/app/lib/stage-storage";
import { ensureConventionReferent } from "@/app/lib/stage-referents-config";
import {
  getStagePeriodsForClass,
  getStageRemindersForClass,
} from "@/app/lib/stage-periods-config";
import { scheduleSummary } from "@/app/lib/stage-schedule";
import { buildSignatureSummary } from "@/app/lib/stage-signature-summary";
import { STAGE_CONVENTION_STATUS_LABELS } from "@/app/lib/stage-types";

export async function GET(req: Request) {
  try {
    const token = new URL(req.url).searchParams.get("token")?.trim();
    if (!token) return NextResponse.json({ error: "Jeton manquant." }, { status: 400 });

    const convention = await resolveConventionByStudentToken(token);
    if (!convention) return NextResponse.json({ error: "Lien invalide." }, { status: 404 });

    if (!["draft", "admin_rejected"].includes(convention.status)) {
      return NextResponse.json({
        convention: {
          id: convention.id,
          status: convention.status,
          statusLabel: STAGE_CONVENTION_STATUS_LABELS[convention.status],
          stageLabel: convention.stageLabel,
          student: convention.student,
          company: convention.company,
          scheduleSummary: scheduleSummary(convention.schedule),
          signatureSummary: buildSignatureSummary(convention),
          readOnly: true,
        },
      });
    }

    const stageContext = await stageContextForClass(
      convention.student.className,
      convention.schoolYear,
    );

    return NextResponse.json({
      convention,
      readOnly: false,
      stageContext,
      signatureSummary: buildSignatureSummary(convention),
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}

async function stageContextForClass(className: string, schoolYear: string) {
  const [reminders, periods] = await Promise.all([
    getStageRemindersForClass(className, schoolYear),
    getStagePeriodsForClass(className, schoolYear),
  ]);
  return { reminders, periods };
}

export async function PATCH(req: Request) {
  try {
    const body = await req.json();
    const token = String(body.token ?? "").trim();
    if (!token) return NextResponse.json({ error: "Jeton manquant." }, { status: 400 });

    const existing = await resolveConventionByStudentToken(token);
    if (!existing) return NextResponse.json({ error: "Lien invalide." }, { status: 404 });
    if (!["draft", "admin_rejected"].includes(existing.status)) {
      return NextResponse.json({ error: "Cette préconvention n'est plus modifiable." }, { status: 400 });
    }

    let convention = normalizeConventionInput(body.convention ?? body, existing);
    convention = { ...convention, createdBy: { ...convention.createdBy, role: "eleve" } };
    convention = await ensureConventionReferent(convention);

    const action = String(body.action ?? "save");
    if (action === "submit") {
      const result = await submitPreconvention(convention, `${convention.student.firstName} ${convention.student.lastName}`.trim());
      if (!result.ok) return NextResponse.json({ error: result.error }, { status: 400 });
      return NextResponse.json({ success: true, convention: result.convention });
    }

    await saveStageConvention(convention);
    return NextResponse.json({ success: true, convention });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
