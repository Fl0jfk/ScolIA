import { NextResponse } from "next/server";
import { applyConventionSignature } from "@/app/lib/stage-workflow";
import { roleStampsPdf } from "@/app/lib/stage-pdf-sign";
import { loadReferentSignatureBytes } from "@/app/lib/stage-signature-store";
import { getSignTokenRef, getStageConvention } from "@/app/lib/stage-storage";
import { scheduleSummary } from "@/app/lib/stage-schedule";
import { STAGE_SIGNER_ROLE_LABELS } from "@/app/lib/stage-types";
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

    const needsDrawnSignature =
      signature.role === "professeur_referent" && stampsPdf && !hasStoredReferentSignature;

    return NextResponse.json({
      convention: {
        id: convention.id,
        studentName: `${convention.student.firstName} ${convention.student.lastName}`.trim(),
        className: convention.student.className,
        companyName: convention.company.name,
        period: `${convention.schedule.periodStart} → ${convention.schedule.periodEnd}`,
        scheduleSummary: scheduleSummary(convention.schedule),
        hasPdf: Boolean(convention.uploadedPdf?.s3Key),
      },
      signature: {
        role: signature.role,
        roleLabel: STAGE_SIGNER_ROLE_LABELS[signature.role],
        label: signature.label,
        status: signature.status,
        signedAt: signature.signedAt,
        signedBy: signature.signedBy,
      },
      stampsPdf,
      needsDrawnSignature,
      hasStoredReferentSignature,
      pdfUrl: convention.uploadedPdf?.s3Key
        ? `/api/stages/public/sign/pdf?token=${encodeURIComponent(token)}`
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
    const token = String(body.token ?? "").trim();
    const signerName = String(body.signerName ?? "").trim();
    const signaturePngBase64 = String(body.signaturePngBase64 ?? "").trim() || undefined;
    if (!token) return NextResponse.json({ error: "Jeton manquant." }, { status: 400 });

    const result = await applyConventionSignature({ token, signerName, signaturePngBase64 });
    if (!result.ok) return NextResponse.json({ error: result.error }, { status: 400 });
    return NextResponse.json({ success: true, status: result.convention.status });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
