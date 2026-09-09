import { NextResponse } from "next/server";
import {
  applyConventionSignature,
  requestSignConfirmCode,
  resolveSignTokenBySecureCode,
} from "@/app/lib/stage-workflow";
import { roleStampsPdf } from "@/app/lib/stage-pdf-sign";
import { loadReferentSignatureBytes } from "@/app/lib/stage-signature-store";
import { getSignTokenRef, getStageConvention } from "@/app/lib/stage-storage";
import {
  formatDaySlotLabel,
  formatPeriodRangeFr,
  scheduleSummary,
  STAGE_WEEKDAY_LABELS,
} from "@/app/lib/stage-schedule";
import {
  isExternalStageSignerRole,
  STAGE_SIGNER_ROLE_LABELS,
  type StageSignMethod,
} from "@/app/lib/stage-types";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";

const signPublicLimiter = createMemoryRateLimiter({
  windowMs: 10 * 60 * 1000,
  max: 20,
});

export async function GET(req: Request) {
  try {
    const token = new URL(req.url).searchParams.get("token")?.trim();
    if (!token) return NextResponse.json({ error: "Jeton manquant." }, { status: 400 });

    const ref = await getSignTokenRef(token);
    if (!ref) return NextResponse.json({ error: "Lien invalide." }, { status: 404 });

    const convention = await getStageConvention(ref.conventionId);
    if (!convention) return NextResponse.json({ error: "Convention introuvable." }, { status: 404 });

    const signature = convention.signatures.find((s) => s.id === ref.signatureId);
    if (!signature) return NextResponse.json({ error: "Signature introuvable." }, { status: 404 });

    const stampsPdf = roleStampsPdf(signature.role);
    let hasStoredReferentSignature = false;
    if (signature.role === "professeur_referent" && convention.teacherReferent.userId) {
      const stored = await loadReferentSignatureBytes(convention.teacherReferent.userId);
      hasStoredReferentSignature = Boolean(stored?.length);
    }

    const isExternal = isExternalStageSignerRole(signature.role);
    const needsDrawnSignature =
      !isExternal && signature.role === "professeur_referent" && stampsPdf && !hasStoredReferentSignature;

    const scheduleDays = convention.schedule.days.map((day) => {
      const title = day.date
        ? new Date(`${day.date}T12:00:00`).toLocaleDateString("fr-FR", {
            weekday: "long",
            day: "numeric",
            month: "long",
          })
        : day.weekday
          ? STAGE_WEEKDAY_LABELS[day.weekday]
          : "Jour";
      let hours = "—";
      if (!day.hasLunchBreak && day.fullDayStart && day.fullDayEnd) {
        hours = `${day.fullDayStart} – ${day.fullDayEnd}`;
      } else {
        const parts: string[] = [];
        if (day.morningStart && day.morningEnd) {
          parts.push(`${day.morningStart} – ${day.morningEnd}`);
        }
        if (day.afternoonStart && day.afternoonEnd) {
          parts.push(`${day.afternoonStart} – ${day.afternoonEnd}`);
        }
        if (parts.length) hours = parts.join("  ·  ");
      }
      return {
        title,
        hours,
        label: formatDaySlotLabel(day),
      };
    });

    return NextResponse.json({
      convention: {
        id: convention.id,
        studentName: `${convention.student.firstName} ${convention.student.lastName}`.trim(),
        className: convention.student.className,
        companyName: convention.company.name,
        period: `${convention.schedule.periodStart} → ${convention.schedule.periodEnd}`,
        periodLabel: formatPeriodRangeFr(
          convention.schedule.periodStart,
          convention.schedule.periodEnd,
        ),
        scheduleSummary: scheduleSummary(convention.schedule),
        scheduleDays,
        hasPdf: Boolean(convention.uploadedPdf?.s3Key),
      },
      signature: {
        role: signature.role,
        roleLabel: STAGE_SIGNER_ROLE_LABELS[signature.role],
        label: signature.label,
        status: signature.status,
        signedAt: signature.signedAt,
        signedBy: signature.signedBy,
        reviewStatus: signature.reviewStatus,
        signMethod: signature.signMethod,
      },
      isExternalSigner: isExternal,
      stampsPdf,
      needsDrawnSignature,
      hasStoredReferentSignature,
      pdfUrl: convention.uploadedPdf?.s3Key
        ? `/api/stages/public/sign/pdf?token=${encodeURIComponent(token)}`
        : null,
      pdfDownloadUrl: convention.uploadedPdf?.s3Key
        ? `/api/stages/public/sign/pdf?token=${encodeURIComponent(token)}&download=1`
        : null,
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}

export async function POST(req: Request) {
  try {
    if (!(await signPublicLimiter.allow(clientIpFromRequest(req)))) {
      return NextResponse.json(
        { error: "Trop de tentatives. Réessayez dans quelques minutes." },
        { status: 429 },
      );
    }

    const body = await req.json();
    const action = String(body.action ?? "sign");

    if (action === "resolve_code") {
      const email = String(body.email ?? "").trim();
      const code = String(body.code ?? "").trim();
      const token = await resolveSignTokenBySecureCode(email, code);
      if (!token) {
        return NextResponse.json({ error: "E-mail ou code incorrect." }, { status: 404 });
      }
      return NextResponse.json({ token });
    }

    if (action === "request_confirm_code") {
      const token = String(body.token ?? "").trim();
      if (!token) return NextResponse.json({ error: "Jeton manquant." }, { status: 400 });
      const result = await requestSignConfirmCode(token);
      if (!result.ok) return NextResponse.json({ error: result.error }, { status: 400 });
      if (!result.sent) {
        return NextResponse.json(
          {
            error:
              result.reason === "smtp"
                ? "Envoi e-mail indisponible (SMTP non configuré)."
                : "Impossible d'envoyer le code par e-mail.",
          },
          { status: 502 },
        );
      }
      return NextResponse.json({ success: true, sent: true });
    }

    const token = String(body.token ?? "").trim();
    const signerName = String(body.signerName ?? "").trim();
    const signaturePngBase64 = String(body.signaturePngBase64 ?? "").trim() || undefined;
    const paperPdfBase64 = String(body.paperPdfBase64 ?? "").trim() || undefined;
    const paperFileName = String(body.paperFileName ?? "").trim() || undefined;
    const confirmCode = String(body.confirmCode ?? "").trim() || undefined;
    const signMethod = String(body.signMethod ?? "").trim() as StageSignMethod | "";
    if (!token) return NextResponse.json({ error: "Jeton manquant." }, { status: 400 });

    const result = await applyConventionSignature({
      token,
      signerName,
      signaturePngBase64,
      paperPdfBase64,
      paperFileName,
      confirmCode,
      signMethod: signMethod || undefined,
    });
    if (!result.ok) return NextResponse.json({ error: result.error }, { status: 400 });
    return NextResponse.json({
      success: true,
      status: result.convention.status,
      reviewStatus: result.convention.signatures.find((s) => s.signToken === token)?.reviewStatus,
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
