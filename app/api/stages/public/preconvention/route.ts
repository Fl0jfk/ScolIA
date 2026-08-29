import { NextResponse } from "next/server";
import { createPublicPreconventionDraft } from "@/app/lib/stage-workflow";
import { verifyStudentForPreconvention } from "@/app/lib/stage-student-identity";
import { buildStudentStageDossier } from "@/app/lib/stage-student-dossier";
import {
  getStagePeriodsForClass,
  getStageRemindersForClass,
  isClassEligibleForStage,
} from "@/app/lib/stage-periods-config";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";

const preconventionLimiter = createMemoryRateLimiter({
  windowMs: 10 * 60 * 1000,
  max: 40,
});

const identityLimiter = createMemoryRateLimiter({
  windowMs: 15 * 60 * 1000,
  max: 20,
});

const GENERIC_IDENTITY_ERROR =
  "Identifiant ou date de naissance incorrects. Vérifiez les informations figurant sur le bulletin ou dans Pronote, ou contactez le secrétariat.";

async function verifyAndLoadStudent(ine: string, dateNaissance: string) {
  const verified = await verifyStudentForPreconvention({ ine, dateNaissance });
  if (!verified.ok) return { ok: false as const };

  const { eleve, ...student } = verified.student;
  const eligibility = await isClassEligibleForStage(student.className);
  if (!eligibility.ok) {
    return { ok: false as const, error: eligibility.reason, status: 403 as const };
  }

  const [dossier, reminders, periods] = await Promise.all([
    buildStudentStageDossier(student),
    getStageRemindersForClass(student.className),
    getStagePeriodsForClass(student.className),
  ]);

  return {
    ok: true as const,
    eleve,
    student,
    dossier,
    stageContext: { reminders, periods },
    parent1Email:
      eleve.parent1Email?.trim() ||
      eleve.parentEmail?.trim() ||
      undefined,
    parent2Email: eleve.parent2Email?.trim() || undefined,
  };
}

/** Identification élève → tableau de bord multi-stages (sans créer de brouillon). */
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
    const action = String(body.action ?? "identify");

    if (!ine || !dateNaissance) {
      return NextResponse.json(
        { error: "L'identifiant élève (INE) et la date de naissance sont obligatoires." },
        { status: 400 },
      );
    }

    if (!(await identityLimiter.allow(clientIpFromRequest(req)))) {
      return NextResponse.json({ error: GENERIC_IDENTITY_ERROR }, { status: 403 });
    }

    const loaded = await verifyAndLoadStudent(ine, dateNaissance);
    if (!loaded.ok) {
      if ("error" in loaded && loaded.error) {
        return NextResponse.json({ error: loaded.error }, { status: loaded.status ?? 403 });
      }
      return NextResponse.json({ error: GENERIC_IDENTITY_ERROR }, { status: 403 });
    }

    if (action === "identify") {
      return NextResponse.json({
        success: true,
        mode: "dashboard",
        studentPreview: {
          firstName: loaded.student.firstName,
          lastName: loaded.student.lastName,
          className: loaded.student.className,
          parent1Email: loaded.parent1Email || null,
          parent2Email: loaded.parent2Email || null,
          parentPhone:
            loaded.eleve.parent1Phone?.trim() ||
            loaded.eleve.parentPhone?.trim() ||
            null,
          parent2Phone: loaded.eleve.parent2Phone?.trim() || null,
          studentEmail: loaded.eleve.email?.trim() || null,
        },
        dossier: {
          schoolYear: loaded.dossier.schoolYear,
          conventions: loaded.dossier.conventions,
          availablePeriods: loaded.dossier.availablePeriods,
          canCreateNew: loaded.dossier.canCreateNew,
        },
        stageContext: loaded.stageContext,
      });
    }

    if (action === "create") {
      if (!loaded.dossier.canCreateNew) {
        return NextResponse.json(
          {
            error:
              "Vous avez déjà ouvert tous les stages prévus pour cette année. Contactez le secrétariat si vous devez en ajouter un.",
          },
          { status: 400 },
        );
      }

      const periodId = String(body.periodId ?? "").trim() || undefined;
      const selectedPeriod = periodId
        ? loaded.dossier.availablePeriods.find((p) => p.id === periodId)
        : undefined;

      if (periodId && selectedPeriod?.used) {
        return NextResponse.json(
          { error: "Cette période de stage est déjà utilisée par un autre dossier." },
          { status: 400 },
        );
      }

      const parent1Override = String(body.parent1Email ?? "").trim() || undefined;
      const parent2Override = String(body.parent2Email ?? "").trim() || undefined;

      const { convention, studentLink } = await createPublicPreconventionDraft({
        ...loaded.student,
        email: loaded.eleve.email?.trim() || undefined,
        parent1Email: parent1Override || loaded.parent1Email,
        parent2Email: parent2Override || loaded.parent2Email,
        parentEmail: parent1Override || loaded.parent1Email,
        matchedEleveIne: loaded.eleve.ine,
        stagePeriodId: selectedPeriod?.id,
        stageLabel: selectedPeriod?.label || String(body.stageLabel ?? "").trim() || undefined,
        periodStart: selectedPeriod?.periodStart,
        periodEnd: selectedPeriod?.periodEnd,
      });

      return NextResponse.json({
        success: true,
        mode: "created",
        conventionId: convention.id,
        studentLink,
      });
    }

    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
