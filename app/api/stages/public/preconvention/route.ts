import { NextResponse } from "next/server";
import { createPublicPreconventionDraft } from "@/app/lib/stage-workflow";
import { verifyStudentForPreconvention } from "@/app/lib/stage-student-identity";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";

const preconventionLimiter = createMemoryRateLimiter({
  windowMs: 10 * 60 * 1000,
  max: 30,
});

const identityLimiter = createMemoryRateLimiter({
  windowMs: 15 * 60 * 1000,
  max: 12,
});

const GENERIC_IDENTITY_ERROR =
  "Identifiant ou date de naissance incorrects. Vérifiez les informations figurant sur le bulletin ou dans Pronote, ou contactez le secrétariat.";

export async function POST(req: Request) {
  try {
    if (!(await preconventionLimiter.allow(clientIpFromRequest(req)))) {
      return NextResponse.json(
        { error: "Trop de tentatives. Réessayez dans quelques minutes." },
        { status: 429 },
      );
    }

    const body = await req.json();
    const ine = String(body.ine ?? "").trim();
    const dateNaissance = String(body.dateNaissance ?? "").trim();

    if (!ine || !dateNaissance) {
      return NextResponse.json(
        { error: "L'identifiant élève (INE) et la date de naissance sont obligatoires." },
        { status: 400 },
      );
    }

    if (!(await identityLimiter.allow(clientIpFromRequest(req)))) {
      return NextResponse.json({ error: GENERIC_IDENTITY_ERROR }, { status: 403 });
    }

    const verified = await verifyStudentForPreconvention({ ine, dateNaissance });
    if (!verified.ok) {
      return NextResponse.json({ error: GENERIC_IDENTITY_ERROR }, { status: 403 });
    }

    const { eleve, ...student } = verified.student;
    const parentEmail =
      eleve.parentEmail?.trim() ||
      eleve.parent1Email?.trim() ||
      eleve.parent2Email?.trim() ||
      undefined;

    const { convention, studentLink } = await createPublicPreconventionDraft({
      ...student,
      email: eleve.email?.trim() || undefined,
      parentEmail,
      matchedEleveIne: eleve.ine,
    });

    return NextResponse.json({
      success: true,
      conventionId: convention.id,
      studentLink,
      studentPreview: {
        firstName: student.firstName,
        lastName: student.lastName,
        className: student.className,
      },
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
