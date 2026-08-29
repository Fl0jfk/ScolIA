import { NextResponse } from "next/server";
import {
  confirmParentEmailVerificationCode,
  normalizeConventionInput,
  resolveConventionByStudentToken,
  sendParentEmailVerificationCode,
  submitPreconvention,
  updateTutorEmailAndResend,
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
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";

const studentLimiter = createMemoryRateLimiter({
  windowMs: 10 * 60 * 1000,
  max: 60,
});

export async function GET(req: Request) {
  try {
    const token = new URL(req.url).searchParams.get("token")?.trim();
    if (!token) return NextResponse.json({ error: "Jeton manquant." }, { status: 400 });

    const convention = await resolveConventionByStudentToken(token);
    if (!convention) return NextResponse.json({ error: "Lien invalide." }, { status: 404 });

    if (!["draft", "admin_rejected"].includes(convention.status)) {
      const stageContext = await stageContextForClass(
        convention.student.className,
        convention.schoolYear,
      );
      return NextResponse.json({
        convention,
        readOnly: true,
        canEditTutorEmail: convention.status === "signatures_pending",
        stageContext,
        signatureSummary: buildSignatureSummary(convention),
        statusLabel: STAGE_CONVENTION_STATUS_LABELS[convention.status],
        scheduleSummary: scheduleSummary(convention.schedule),
      });
    }

    const stageContext = await stageContextForClass(
      convention.student.className,
      convention.schoolYear,
    );

    const parentEmail =
      convention.parentSignerEmail?.trim() ||
      convention.student.parent1Email?.trim() ||
      convention.student.parentEmail?.trim() ||
      "";
    const parentEmailVerified = Boolean(
      convention.parentEmailVerification?.verifiedAt &&
        convention.parentEmailVerification.email.toLowerCase() === parentEmail.toLowerCase(),
    );

    return NextResponse.json({
      convention,
      readOnly: false,
      stageContext,
      signatureSummary: buildSignatureSummary(convention),
      parentEmailVerified,
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
    if (!(await studentLimiter.allow(clientIpFromRequest(req)))) {
      return NextResponse.json(
        { error: "Trop de tentatives. Réessayez dans quelques minutes." },
        { status: 429 },
      );
    }

    const body = await req.json();
    const token = String(body.token ?? "").trim();
    if (!token) return NextResponse.json({ error: "Jeton manquant." }, { status: 400 });

    const existing = await resolveConventionByStudentToken(token);
    if (!existing) return NextResponse.json({ error: "Lien invalide." }, { status: 404 });

    const action = String(body.action ?? "save");

    if (action === "update_tutor_email") {
      if (!["draft", "admin_rejected", "signatures_pending"].includes(existing.status)) {
        return NextResponse.json({ error: "Modification impossible pour ce dossier." }, { status: 400 });
      }
      const result = await updateTutorEmailAndResend({
        convention: existing,
        tutorEmail: String(body.tutorEmail ?? ""),
        tutorName: String(body.tutorName ?? "").trim() || undefined,
      });
      if (!result.ok) return NextResponse.json({ error: result.error }, { status: 400 });
      return NextResponse.json({ success: true, convention: result.convention });
    }

    if (!["draft", "admin_rejected"].includes(existing.status)) {
      return NextResponse.json({ error: "Cette préconvention n'est plus modifiable." }, { status: 400 });
    }

    let convention = normalizeConventionInput(body.convention ?? body, existing);
    convention = {
      ...convention,
      createdBy: { ...convention.createdBy, role: "eleve" },
      parentEmailVerification: existing.parentEmailVerification,
    };
    convention = await ensureConventionReferent(convention);

    if (action === "send_parent_code") {
      const result = await sendParentEmailVerificationCode(convention);
      if (!result.ok) return NextResponse.json({ error: result.error }, { status: 400 });
      if (!result.sent) {
        return NextResponse.json(
          {
            error:
              result.reason === "smtp"
                ? "Envoi d'e-mail indisponible (SMTP non configuré)."
                : "Impossible d'envoyer le code à cette adresse. Vérifiez l'e-mail du responsable.",
          },
          { status: 400 },
        );
      }
      return NextResponse.json({
        success: true,
        convention: result.convention,
        message: "Code envoyé. Vérifiez la boîte mail du responsable légal.",
      });
    }

    if (action === "confirm_parent_code") {
      const result = await confirmParentEmailVerificationCode(
        convention,
        String(body.code ?? ""),
      );
      if (!result.ok) return NextResponse.json({ error: result.error }, { status: 400 });
      return NextResponse.json({
        success: true,
        convention: result.convention,
        parentEmailVerified: true,
      });
    }

    if (action === "submit") {
      const result = await submitPreconvention(
        convention,
        `${convention.student.firstName} ${convention.student.lastName}`.trim(),
      );
      if (!result.ok) return NextResponse.json({ error: result.error }, { status: 400 });
      return NextResponse.json({ success: true, convention: result.convention });
    }

    await saveStageConvention(convention);
    return NextResponse.json({ success: true, convention });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
